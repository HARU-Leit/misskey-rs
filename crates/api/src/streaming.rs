//! WebSocket streaming API.

#![allow(missing_docs)]

use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use crate::middleware::AppState;

/// Streaming query parameters.
#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    /// Access token for authentication.
    #[serde(rename = "i")]
    pub token: Option<String>,
}

/// Stream channel types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StreamChannel {
    /// Home timeline (followed users' posts).
    HomeTimeline,
    /// Local timeline (all local posts).
    LocalTimeline,
    /// Global timeline (all federated posts).
    GlobalTimeline,
    /// Main stream (notifications, etc.).
    Main,
    /// User-specific stream.
    User { user_id: String },
    /// Channel timeline stream.
    Channel { channel_id: String },
}

/// Stream event types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "body", rename_all = "camelCase")]
pub enum StreamEvent {
    /// New note event.
    Note(NoteEvent),
    /// Note deleted event.
    NoteDeleted { id: String },
    /// New notification event.
    Notification(NotificationEvent),
    /// Follow event.
    Followed { id: String, user_id: String },
    /// Unfollow event.
    Unfollowed { id: String, user_id: String },
    /// Mention event.
    Mention(NoteEvent),
}

/// Note event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteEvent {
    pub id: String,
    pub user_id: String,
    pub text: Option<String>,
    pub cw: Option<String>,
    pub visibility: String,
    pub created_at: String,
}

/// Notification event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEvent {
    pub id: String,
    pub r#type: String,
    pub user_id: Option<String>,
    pub note_id: Option<String>,
    pub created_at: String,
}

/// Client-to-server message.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "body", rename_all = "camelCase")]
pub enum ClientMessage {
    /// Connect to a channel.
    Connect {
        channel: String,
        id: String,
        #[serde(default)]
        params: serde_json::Value,
    },
    /// Disconnect from a channel.
    Disconnect { id: String },
    /// Subscribe to a note's thread.
    SubNote { id: String },
    /// Unsubscribe from a note's thread.
    UnsubNote { id: String },
    /// Send a read notification.
    ReadNotification { id: String },
}

/// Server-to-client message.
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "body", rename_all = "camelCase")]
pub enum ServerMessage {
    /// Channel connected.
    Connected { id: String },
    /// Channel event.
    Channel {
        id: String,
        #[serde(rename = "type")]
        event_type: String,
        body: serde_json::Value,
    },
    /// Note updated.
    NoteUpdated {
        id: String,
        #[serde(rename = "type")]
        event_type: String,
        body: serde_json::Value,
    },
}

/// Channel event for streaming to specific channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelEvent {
    /// The channel ID this event belongs to.
    pub channel_id: String,
    /// The stream event.
    pub event: StreamEvent,
}

/// Shared state for streaming.
#[derive(Clone)]
pub struct StreamingState {
    /// Broadcast sender for global events.
    pub global_tx: Arc<broadcast::Sender<StreamEvent>>,
    /// Broadcast sender for local events.
    pub local_tx: Arc<broadcast::Sender<StreamEvent>>,
    /// Broadcast sender for channel-specific events.
    pub channel_tx: Arc<broadcast::Sender<ChannelEvent>>,
}

impl StreamingState {
    /// Create a new streaming state.
    #[must_use]
    pub fn new() -> Self {
        let (global_tx, _) = broadcast::channel(1000);
        let (local_tx, _) = broadcast::channel(1000);
        let (channel_tx, _) = broadcast::channel(1000);

        Self {
            global_tx: Arc::new(global_tx),
            local_tx: Arc::new(local_tx),
            channel_tx: Arc::new(channel_tx),
        }
    }

    /// Publish a note event to the appropriate channels.
    pub fn publish_note(&self, event: NoteEvent) {
        let stream_event = StreamEvent::Note(event.clone());

        // Publish to local timeline if visibility is public
        if event.visibility == "public" {
            let _ = self.local_tx.send(stream_event.clone());
        }

        // Publish to global timeline
        if event.visibility == "public" {
            let _ = self.global_tx.send(stream_event);
        }
    }

    /// Publish a note event to a specific channel timeline.
    pub fn publish_channel_note(&self, channel_id: &str, event: NoteEvent) {
        let stream_event = StreamEvent::Note(event);
        let channel_event = ChannelEvent {
            channel_id: channel_id.to_string(),
            event: stream_event,
        };
        let _ = self.channel_tx.send(channel_event);
    }

    /// Publish a notification event.
    pub fn publish_notification(&self, _user_id: &str, event: NotificationEvent) {
        let stream_event = StreamEvent::Notification(event);
        // For now, publish to global - in production, use user-specific channels
        let _ = self.global_tx.send(stream_event);
    }
}

impl Default for StreamingState {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket handler for streaming.
pub async fn streaming_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<StreamQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!("New streaming connection");

    ws.on_upgrade(move |socket| handle_socket(socket, query, state))
}

/// Handle a WebSocket connection.
async fn handle_socket(socket: WebSocket, query: StreamQuery, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Authenticate if token provided
    let user = if let Some(token) = &query.token {
        match state.user_service.authenticate_by_token(token).await {
            Ok(u) => Some(u),
            Err(e) => {
                warn!("Streaming auth failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    let user_id = user.map(|u| u.id);

    info!(user_id = ?user_id, "Streaming connection established");

    // Subscribe to broadcast channels
    let mut global_rx = state.streaming.global_tx.subscribe();
    let mut local_rx = state.streaming.local_tx.subscribe();
    let mut channel_rx = state.streaming.channel_tx.subscribe();

    // Track connected channels
    let mut connected_channels: std::collections::HashMap<String, StreamChannel> =
        std::collections::HashMap::new();

    loop {
        tokio::select! {
            // Handle incoming messages from client
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(client_msg) => {
                                if let Some(response) = handle_client_message(
                                    client_msg,
                                    &mut connected_channels,
                                    user_id.as_deref(),
                                ).await {
                                    let json = serde_json::to_string(&response).unwrap_or_default();
                                    if sender.send(Message::Text(json.into())).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse client message: {}", e);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("Client closed connection");
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }

            // Handle global timeline events
            Ok(event) = global_rx.recv() => {
                if connected_channels.values().any(|c| matches!(c, StreamChannel::GlobalTimeline))
                    && let Some(channel_id) = find_channel_id(&connected_channels, &StreamChannel::GlobalTimeline) {
                        let msg = event_to_server_message(&channel_id, &event);
                        let json = serde_json::to_string(&msg).unwrap_or_default();
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
            }

            // Handle local timeline events
            Ok(event) = local_rx.recv() => {
                if connected_channels.values().any(|c| matches!(c, StreamChannel::LocalTimeline))
                    && let Some(channel_id) = find_channel_id(&connected_channels, &StreamChannel::LocalTimeline) {
                        let msg = event_to_server_message(&channel_id, &event);
                        let json = serde_json::to_string(&msg).unwrap_or_default();
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
            }

            // Handle channel-specific events
            Ok(channel_event) = channel_rx.recv() => {
                // Find if user is subscribed to this specific channel
                for (conn_id, stream_channel) in &connected_channels {
                    if let StreamChannel::Channel { channel_id } = stream_channel
                        && *channel_id == channel_event.channel_id {
                            let msg = event_to_server_message(conn_id, &channel_event.event);
                            let json = serde_json::to_string(&msg).unwrap_or_default();
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                }
            }
        }
    }

    info!("Streaming connection closed");
}

/// Handle a client message.
async fn handle_client_message(
    msg: ClientMessage,
    connected_channels: &mut std::collections::HashMap<String, StreamChannel>,
    _user_id: Option<&str>,
) -> Option<ServerMessage> {
    match msg {
        ClientMessage::Connect {
            channel,
            id,
            params,
        } => {
            let stream_channel = match channel.as_str() {
                "homeTimeline" => StreamChannel::HomeTimeline,
                "localTimeline" => StreamChannel::LocalTimeline,
                "globalTimeline" => StreamChannel::GlobalTimeline,
                "main" => StreamChannel::Main,
                "channel" => {
                    // Extract channelId from params
                    let channel_id = params
                        .get("channelId")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    if let Some(channel_id) = channel_id {
                        StreamChannel::Channel { channel_id }
                    } else {
                        warn!("Channel connection without channelId param");
                        return None;
                    }
                }
                _ => {
                    warn!("Unknown channel: {}", channel);
                    return None;
                }
            };

            let channel_desc = match &stream_channel {
                StreamChannel::Channel { channel_id } => format!("channel:{channel_id}"),
                _ => channel,
            };

            connected_channels.insert(id.clone(), stream_channel);
            info!(channel = %channel_desc, id = %id, "Channel connected");

            Some(ServerMessage::Connected { id })
        }
        ClientMessage::Disconnect { id } => {
            connected_channels.remove(&id);
            info!(id = %id, "Channel disconnected");
            None
        }
        ClientMessage::SubNote { id } => {
            info!(note_id = %id, "Subscribed to note");
            None
        }
        ClientMessage::UnsubNote { id } => {
            info!(note_id = %id, "Unsubscribed from note");
            None
        }
        ClientMessage::ReadNotification { id } => {
            info!(notification_id = %id, "Notification marked as read");
            None
        }
    }
}

/// Find the channel ID for a given stream channel.
fn find_channel_id(
    channels: &std::collections::HashMap<String, StreamChannel>,
    target: &StreamChannel,
) -> Option<String> {
    channels
        .iter()
        .find(|(_, v)| *v == target)
        .map(|(k, _)| k.clone())
}

/// Convert a stream event to a server message.
fn event_to_server_message(channel_id: &str, event: &StreamEvent) -> ServerMessage {
    let (event_type, body) = match event {
        StreamEvent::Note(note) => ("note", serde_json::to_value(note).unwrap_or_default()),
        StreamEvent::NoteDeleted { id } => ("noteDeleted", serde_json::json!({ "id": id })),
        StreamEvent::Notification(notif) => (
            "notification",
            serde_json::to_value(notif).unwrap_or_default(),
        ),
        StreamEvent::Followed { id, user_id } => (
            "followed",
            serde_json::json!({ "id": id, "userId": user_id }),
        ),
        StreamEvent::Unfollowed { id, user_id } => (
            "unfollowed",
            serde_json::json!({ "id": id, "userId": user_id }),
        ),
        StreamEvent::Mention(note) => ("mention", serde_json::to_value(note).unwrap_or_default()),
    };

    ServerMessage::Channel {
        id: channel_id.to_string(),
        event_type: event_type.to_string(),
        body,
    }
}
