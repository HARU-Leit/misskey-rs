//! Server-Sent Events (SSE) for real-time updates.
//!
//! Provides SSE streams for notifications and timeline updates.

#![allow(missing_docs)]

use std::convert::Infallible;
use std::time::Duration;

use axum::{
    Router,
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
};
use futures::stream::{self, Stream};
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::{extractors::AuthUser, middleware::AppState};

/// SSE event types.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SseEvent {
    /// New note on timeline.
    Note {
        id: String,
        user_id: String,
        text: Option<String>,
    },
    /// Note deleted.
    NoteDeleted { id: String },
    /// New notification.
    Notification {
        id: String,
        #[serde(rename = "notificationType")]
        notification_type: String,
        user_id: Option<String>,
        note_id: Option<String>,
    },
    /// New follower.
    Followed { user_id: String },
    /// User unfollowed.
    Unfollowed { user_id: String },
    /// New reaction on note.
    Reaction {
        note_id: String,
        user_id: String,
        reaction: String,
    },
    /// Mention in note.
    Mention { note_id: String, user_id: String },
    /// Connection established.
    Connected,
}

/// SSE broadcast channels for different streams.
#[derive(Clone)]
pub struct SseBroadcaster {
    /// Global timeline events.
    pub global: broadcast::Sender<SseEvent>,
    /// Local timeline events.
    pub local: broadcast::Sender<SseEvent>,
    /// User-specific events (keyed by user ID).
    user_channels: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, broadcast::Sender<SseEvent>>>,
    >,
}

impl SseBroadcaster {
    /// Create a new SSE broadcaster.
    #[must_use]
    pub fn new() -> Self {
        let (global, _) = broadcast::channel(1000);
        let (local, _) = broadcast::channel(1000);

        Self {
            global,
            local,
            user_channels: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    /// Get or create a user-specific channel.
    pub async fn user_channel(&self, user_id: &str) -> broadcast::Sender<SseEvent> {
        let mut channels = self.user_channels.write().await;

        if let Some(sender) = channels.get(user_id)
            && sender.receiver_count() > 0
        {
            return sender.clone();
        }

        let (sender, _) = broadcast::channel(100);
        channels.insert(user_id.to_string(), sender.clone());
        sender
    }

    /// Broadcast an event to the global timeline.
    pub fn broadcast_global(&self, event: SseEvent) {
        let _ = self.global.send(event);
    }

    /// Broadcast an event to the local timeline.
    pub fn broadcast_local(&self, event: SseEvent) {
        let _ = self.local.send(event);
    }

    /// Broadcast an event to a specific user.
    pub async fn broadcast_to_user(&self, user_id: &str, event: SseEvent) {
        let channels = self.user_channels.read().await;
        if let Some(sender) = channels.get(user_id) {
            let _ = sender.send(event);
        }
    }

    /// Clean up inactive user channels.
    pub async fn cleanup(&self) {
        let mut channels = self.user_channels.write().await;
        channels.retain(|_, sender| sender.receiver_count() > 0);
    }
}

impl Default for SseBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Global timeline SSE stream.
async fn global_timeline(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.sse_broadcaster.global.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().map(|event| {
            Ok(Event::default()
                .json_data(&event)
                .unwrap_or_else(|_| Event::default().data("error")))
        })
    });

    // Add initial connected event
    let initial = stream::once(async {
        Ok(Event::default()
            .json_data(&SseEvent::Connected)
            .unwrap_or_else(|_| Event::default().data("connected")))
    });

    Sse::new(initial.chain(stream)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("ping"),
    )
}

/// Local timeline SSE stream.
async fn local_timeline(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.sse_broadcaster.local.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().map(|event| {
            Ok(Event::default()
                .json_data(&event)
                .unwrap_or_else(|_| Event::default().data("error")))
        })
    });

    let initial = stream::once(async {
        Ok(Event::default()
            .json_data(&SseEvent::Connected)
            .unwrap_or_else(|_| Event::default().data("connected")))
    });

    Sse::new(initial.chain(stream)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("ping"),
    )
}

/// User-specific SSE stream (notifications, mentions, etc.).
async fn user_stream(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let sender = state.sse_broadcaster.user_channel(&user.id).await;
    let rx = sender.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(|result| {
        result.ok().map(|event| {
            Ok(Event::default()
                .json_data(&event)
                .unwrap_or_else(|_| Event::default().data("error")))
        })
    });

    let initial = stream::once(async {
        Ok(Event::default()
            .json_data(&SseEvent::Connected)
            .unwrap_or_else(|_| Event::default().data("connected")))
    });

    Sse::new(initial.chain(stream)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("ping"),
    )
}

/// Create SSE router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/global", get(global_timeline))
        .route("/local", get(local_timeline))
        .route("/user", get(user_stream))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_broadcaster_new() {
        let broadcaster = SseBroadcaster::new();
        assert_eq!(broadcaster.global.receiver_count(), 0);
        assert_eq!(broadcaster.local.receiver_count(), 0);
    }

    #[tokio::test]
    async fn test_sse_broadcaster_broadcast_global() {
        let broadcaster = SseBroadcaster::new();
        let mut rx = broadcaster.global.subscribe();

        broadcaster.broadcast_global(SseEvent::Connected);

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, SseEvent::Connected));
    }

    #[tokio::test]
    async fn test_sse_broadcaster_user_channel() {
        let broadcaster = SseBroadcaster::new();

        let sender1 = broadcaster.user_channel("user1").await;
        let sender2 = broadcaster.user_channel("user1").await;

        // Should get the same channel
        assert_eq!(sender1.receiver_count(), sender2.receiver_count());
    }

    #[test]
    fn test_sse_event_serialization() {
        let event = SseEvent::Note {
            id: "123".to_string(),
            user_id: "user1".to_string(),
            text: Some("Hello".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"note\""));
        assert!(json.contains("\"id\":\"123\""));
    }

    #[test]
    fn test_notification_event_serialization() {
        let event = SseEvent::Notification {
            id: "notif1".to_string(),
            notification_type: "follow".to_string(),
            user_id: Some("user1".to_string()),
            note_id: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"notification\""));
        assert!(json.contains("\"notificationType\":\"follow\""));
    }
}
