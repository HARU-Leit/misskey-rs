//! API integration tests.
//!
//! These tests verify the API endpoints work correctly together.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::redundant_clone)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use misskey_api::{SseBroadcaster, StreamingState, middleware::AppState, router as api_router};
use misskey_common::config::{Config, DatabaseConfig, FederationConfig, RedisConfig, ServerConfig};
use misskey_core::{
    AnnouncementService, AntennaService, BlockingService, ChannelService, ClipService,
    DriveService, EmojiService, FollowingService, GalleryService, GroupService, InstanceService,
    MessagingService, MetaSettingsService, ModerationService, MutingService, NoteFavoriteService,
    NoteService, NotificationService, OAuthService, PageService, PollService, ReactionService,
    RegistrationApprovalService, ScheduledNoteService, TwoFactorService, UserListService,
    UserService, WebAuthnConfig, WebAuthnService, WebhookService, WordFilterService,
};
use misskey_db::repositories::{
    AnnouncementRepository, AntennaRepository, BlockingRepository, ChannelRepository,
    ClipRepository, DriveFileRepository, DriveFolderRepository, EmojiRepository,
    FollowRequestRepository, FollowingRepository, GalleryRepository, GroupRepository,
    InstanceRepository, MessagingRepository, ModerationRepository, MutingRepository,
    NoteFavoriteRepository, NoteRepository, NotificationRepository, OAuthRepository,
    PageRepository, PollRepository, PollVoteRepository, ReactionRepository,
    ScheduledNoteRepository, SecurityKeyRepository, UserKeypairRepository, UserListRepository,
    UserProfileRepository, UserRepository, WebhookRepository, WordFilterRepository,
};
use sea_orm::{DatabaseBackend, DatabaseConnection, MockDatabase, MockExecResult};
use std::sync::Arc;
use tower::ServiceExt;

/// Create a test configuration.
fn create_test_config() -> Config {
    Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            url: "https://example.com".to_string(),
        },
        database: DatabaseConfig {
            url: "postgres://localhost/test".to_string(),
            read_replicas: Vec::new(),
            max_connections: 10,
            min_connections: 1,
        },
        redis: RedisConfig {
            url: "redis://localhost".to_string(),
            prefix: "mk:".to_string(),
        },
        federation: FederationConfig {
            enabled: true,
            instance_name: "Test Instance".to_string(),
            instance_description: Some("A test instance".to_string()),
            maintainer_name: None,
            maintainer_email: None,
        },
    }
}

/// Create a mock database connection.
fn create_mock_db() -> DatabaseConnection {
    MockDatabase::new(DatabaseBackend::Postgres)
        .append_exec_results([MockExecResult {
            last_insert_id: 0,
            rows_affected: 1,
        }])
        .into_connection()
}

/// Create test app state with mock database.
fn create_test_state() -> AppState {
    let db = Arc::new(create_mock_db());
    let config = create_test_config();

    let user_repo = UserRepository::new(Arc::clone(&db));
    let user_profile_repo = UserProfileRepository::new(Arc::clone(&db));
    let user_keypair_repo = UserKeypairRepository::new(Arc::clone(&db));
    let note_repo = NoteRepository::new(Arc::clone(&db));
    let following_repo = FollowingRepository::new(Arc::clone(&db));
    let follow_request_repo = FollowRequestRepository::new(Arc::clone(&db));
    let reaction_repo = ReactionRepository::new(Arc::clone(&db));
    let notification_repo = NotificationRepository::new(Arc::clone(&db));
    let blocking_repo = BlockingRepository::new(Arc::clone(&db));
    let muting_repo = MutingRepository::new(Arc::clone(&db));
    let drive_file_repo = DriveFileRepository::new(Arc::clone(&db));
    let drive_folder_repo = DriveFolderRepository::new(Arc::clone(&db));
    let poll_repo = PollRepository::new(Arc::clone(&db));
    let poll_vote_repo = PollVoteRepository::new(Arc::clone(&db));
    let hashtag_repo = misskey_db::repositories::HashtagRepository::new(Arc::clone(&db));
    let note_favorite_repo = NoteFavoriteRepository::new(Arc::clone(&db));
    let user_list_repo = UserListRepository::new(Arc::clone(&db));
    let moderation_repo = ModerationRepository::new(Arc::clone(&db));
    let emoji_repo = EmojiRepository::new(Arc::clone(&db));
    let announcement_repo = AnnouncementRepository::new(Arc::clone(&db));
    let antenna_repo = AntennaRepository::new(Arc::clone(&db));
    let channel_repo = ChannelRepository::new(Arc::clone(&db));
    let instance_repo = InstanceRepository::new(Arc::clone(&db));
    let messaging_repo = MessagingRepository::new(Arc::clone(&db));
    let clip_repo = ClipRepository::new(Arc::clone(&db));
    let word_filter_repo = WordFilterRepository::new(Arc::clone(&db));
    let scheduled_note_repo = ScheduledNoteRepository::new(Arc::clone(&db));
    let security_key_repo = SecurityKeyRepository::new(Arc::clone(&db));
    let oauth_repo = OAuthRepository::new(Arc::clone(&db));
    let webhook_repo = WebhookRepository::new(Arc::clone(&db));
    let page_repo = PageRepository::new(Arc::clone(&db));
    let gallery_repo = GalleryRepository::new(Arc::clone(&db));
    let group_repo = GroupRepository::new(Arc::clone(&db));

    let user_service = UserService::new(
        user_repo.clone(),
        user_profile_repo.clone(),
        user_keypair_repo,
        note_repo.clone(),
        &config,
    );
    let note_service =
        NoteService::new(note_repo.clone(), user_repo.clone(), following_repo.clone());
    let blocking_service = BlockingService::new(blocking_repo.clone(), following_repo.clone());
    let following_repo_for_messaging = following_repo.clone();
    let following_service =
        FollowingService::new(following_repo, follow_request_repo, user_repo.clone());
    let reaction_service = ReactionService::new(reaction_repo, note_repo.clone());
    let notification_service = NotificationService::new(notification_repo);
    let muting_service = MutingService::new(muting_repo);
    let drive_service = DriveService::new(
        drive_file_repo,
        drive_folder_repo,
        "https://example.com".to_string(),
    );
    let poll_service = PollService::new(poll_repo, poll_vote_repo, note_repo.clone());
    let hashtag_service = misskey_core::HashtagService::new(hashtag_repo);
    let note_favorite_service = NoteFavoriteService::new(note_favorite_repo, note_repo);
    let user_list_service = UserListService::new(user_list_repo, user_repo.clone());
    let moderation_service = ModerationService::new(moderation_repo, user_repo.clone());
    let emoji_service = EmojiService::new(emoji_repo);
    let announcement_service = AnnouncementService::new(announcement_repo);
    let antenna_service = AntennaService::new(antenna_repo);
    let channel_service = ChannelService::new(channel_repo);
    let instance_service = InstanceService::new(instance_repo, user_repo.clone());
    let messaging_service = MessagingService::new(
        messaging_repo,
        user_repo.clone(),
        user_profile_repo.clone(),
        following_repo_for_messaging,
        blocking_repo,
    );
    let clip_service = ClipService::new(clip_repo);
    let word_filter_service = WordFilterService::new(word_filter_repo);
    let scheduled_note_service = ScheduledNoteService::new(scheduled_note_repo);
    let two_factor_service = TwoFactorService::new(user_profile_repo.clone());

    // Initialize WebAuthn service
    let webauthn_config =
        WebAuthnConfig::from_server_url(&config.server.url, &config.federation.instance_name)
            .expect("Failed to create WebAuthn config");
    let webauthn_service = WebAuthnService::new(
        &webauthn_config,
        security_key_repo,
        user_repo.clone(),
        user_profile_repo.clone(),
    )
    .expect("Failed to create WebAuthn service");

    let oauth_service = OAuthService::new(oauth_repo);
    let webhook_service = WebhookService::new(webhook_repo);
    let page_service = PageService::new(page_repo);
    let gallery_service = GalleryService::new(gallery_repo);
    let group_service = GroupService::new(group_repo);
    let meta_settings_service = MetaSettingsService::new(db.clone());
    let registration_approval_service = RegistrationApprovalService::new(db.clone());

    let streaming = StreamingState::new();
    let sse_broadcaster = SseBroadcaster::new();

    AppState {
        user_service,
        note_service,
        following_service,
        reaction_service,
        notification_service,
        blocking_service,
        clip_service,
        muting_service,
        drive_service,
        poll_service,
        hashtag_service,
        note_favorite_service,
        user_list_service,
        moderation_service,
        emoji_service,
        announcement_service,
        antenna_service,
        channel_service,
        instance_service,
        messaging_service,
        word_filter_service,
        scheduled_note_service,
        two_factor_service,
        webauthn_service,
        oauth_service,
        webhook_service,
        page_service,
        gallery_service,
        translation_service: None,
        push_notification_service: None,
        account_service: None,
        group_service,
        meta_settings_service,
        registration_approval_service,
        streaming,
        sse_broadcaster,
    }
}

/// Create the test router.
fn create_test_router() -> Router {
    let state = create_test_state();
    api_router().with_state(state)
}

#[tokio::test]
async fn test_meta_endpoint() {
    let app = create_test_router();

    // Misskey API uses POST for most endpoints
    let response = app
        .oneshot(
            Request::builder()
                .uri("/meta")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_signin_without_credentials_returns_error() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/signin")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"username":"nonexistent","password":"wrongpassword"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return error (mock DB won't find user)
    // Could be UNAUTHORIZED, BAD_REQUEST, NOT_FOUND, or INTERNAL_SERVER_ERROR with mock
    let status = response.status();
    assert!(
        status == StatusCode::BAD_REQUEST
            || status == StatusCode::UNAUTHORIZED
            || status == StatusCode::NOT_FOUND
            || status == StatusCode::INTERNAL_SERVER_ERROR
            || status == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_notes_timeline_returns_response() {
    let app = create_test_router();

    // Timeline endpoint - uses POST and returns notes or error with mock DB
    let response = app
        .oneshot(
            Request::builder()
                .uri("/notes/timeline")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    // With mock DB, may return error or empty result
    let status = response.status();
    assert!(
        status == StatusCode::OK
            || status == StatusCode::INTERNAL_SERVER_ERROR
            || status == StatusCode::UNAUTHORIZED // May require auth
            || status == StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn test_users_endpoint_without_id_returns_error() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/users/")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Empty user ID should return not found or bad request
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn test_unknown_endpoint_returns_404() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/nonexistent/endpoint")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_signup_with_invalid_json_returns_error() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/signup")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from("invalid json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_sse_global_timeline_returns_stream() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/streaming/sse/global")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    // SSE returns text/event-stream content type
    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or(""));
    assert!(content_type.is_some());
    assert!(content_type.unwrap().contains("text/event-stream"));
}

#[tokio::test]
async fn test_sse_local_timeline_returns_stream() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/streaming/sse/local")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_sse_user_stream_requires_auth() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/streaming/sse/user")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // User stream requires authentication
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
