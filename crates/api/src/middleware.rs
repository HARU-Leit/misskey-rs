//! API middleware.

#![allow(missing_docs)]

use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::Next,
    response::Response,
};
use misskey_core::{AccountService, AnnouncementService, AntennaService, BlockingService, ChannelService, ClipService, DriveService, EmojiService, FollowingService, GalleryService, GroupService, HashtagService, InstanceService, MessagingService, MetaSettingsService, ModerationService, MutingService, NoteFavoriteService, NoteService, NotificationService, OAuthService, PageService, PollService, PushNotificationService, ReactionService, RegistrationApprovalService, ScheduledNoteService, TranslationService, TwoFactorService, UserListService, UserService, WebAuthnService, WebhookService, WordFilterService};

use crate::sse::SseBroadcaster;
use crate::streaming::StreamingState;

/// Application state.
#[derive(Clone)]
pub struct AppState {
    pub user_service: UserService,
    pub note_service: NoteService,
    pub following_service: FollowingService,
    pub reaction_service: ReactionService,
    pub notification_service: NotificationService,
    pub blocking_service: BlockingService,
    pub clip_service: ClipService,
    pub muting_service: MutingService,
    pub drive_service: DriveService,
    pub poll_service: PollService,
    pub hashtag_service: HashtagService,
    pub note_favorite_service: NoteFavoriteService,
    pub user_list_service: UserListService,
    pub moderation_service: ModerationService,
    pub emoji_service: EmojiService,
    pub announcement_service: AnnouncementService,
    pub antenna_service: AntennaService,
    pub channel_service: ChannelService,
    pub instance_service: InstanceService,
    pub messaging_service: MessagingService,
    pub word_filter_service: WordFilterService,
    pub scheduled_note_service: ScheduledNoteService,
    pub two_factor_service: TwoFactorService,
    pub webauthn_service: WebAuthnService,
    pub oauth_service: OAuthService,
    pub webhook_service: WebhookService,
    pub page_service: PageService,
    pub gallery_service: GalleryService,
    pub translation_service: Option<TranslationService>,
    pub push_notification_service: Option<PushNotificationService>,
    pub account_service: Option<AccountService>,
    pub group_service: GroupService,
    pub meta_settings_service: MetaSettingsService,
    pub registration_approval_service: RegistrationApprovalService,
    pub streaming: StreamingState,
    pub sse_broadcaster: SseBroadcaster,
}

/// Authentication middleware.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    // Try to extract token from header
    if let Some(auth_header) = req.headers().get("Authorization")
        && let Ok(auth_str) = auth_header.to_str()
            && let Some(token) = auth_str.strip_prefix("Bearer ") {
                // Authenticate user by token
                if let Ok(user) = state.user_service.authenticate_by_token(token).await {
                    req.extensions_mut().insert(user);
                }
            }

    // Also check i query parameter (Misskey compatibility)
    // This would require parsing the query string or request body

    next.run(req).await
}
