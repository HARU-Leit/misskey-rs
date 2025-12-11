//! Misskey-rs server entry point.

use std::net::SocketAddr;
use std::sync::Arc;

use apalis::prelude::*;
use axum::{
    Router, middleware,
    routing::{get, post},
};
use misskey_api::{
    SseBroadcaster, StreamingState, middleware::AppState, rate_limit::RateLimiterState,
    router as api_router, streaming_handler,
};
use misskey_common::Config;
use misskey_core::{
    AccountService, AnnouncementService, AntennaService, BlockingService, ChannelService,
    ClipService, DeliveryService, DriveService, EmojiService, FollowingService, GalleryService,
    GroupService, InstanceService, MessagingService, MetaSettingsService, ModerationService,
    MutingService, NoteFavoriteService, NoteService, NotificationService, OAuthService,
    PageService, PollService, ReactionService, RegistrationApprovalService, ScheduledNoteService,
    TwoFactorService, UserListService, UserService, WebAuthnConfig, WebAuthnService,
    WebhookService, WordFilterService,
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
use misskey_federation::{
    ClipCollectionState, CollectionState, InboxState, NodeInfoState, UserApState, WebfingerState,
    clip_handler, clips_list_handler, followers_handler, following_handler, inbox_handler,
    nodeinfo_2_1, outbox_handler, user_handler, user_inbox_handler, webfinger_handler,
    well_known_nodeinfo,
};
use misskey_queue::workers::{DeliverContext, deliver_worker};
use misskey_queue::{DeliverJob, RedisDeliveryService};
use fred::prelude::*;
use sea_orm::{ConnectOptions, Database};
use tokio::signal;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

/// Waits for a shutdown signal (SIGINT or SIGTERM).
///
/// On Unix systems, this listens for both SIGINT (Ctrl+C) and SIGTERM.
/// On Windows, this only listens for Ctrl+C.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            info!("Received SIGINT, initiating graceful shutdown...");
        },
        () = terminate => {
            info!("Received SIGTERM, initiating graceful shutdown...");
        },
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "misskey=debug,tower_http=debug".into()),
        )
        .init();

    info!("Starting misskey-rs server...");

    // Load configuration
    let config = Config::load()?;

    // Connect to database
    let mut db_opts = ConnectOptions::new(&config.database.url);
    db_opts
        .max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections);

    let db = Database::connect(db_opts).await?;
    info!("Connected to database");

    // Run migrations
    info!("Running database migrations...");
    misskey_db::migrate(&db).await?;
    info!("Migrations completed");

    // Connect to Redis and initialize job queue
    info!("Connecting to Redis...");
    let redis_client =
        redis::Client::open(config.redis.url.as_str()).expect("Failed to create Redis client");
    let redis_conn = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("Failed to connect to Redis");
    let redis_storage = apalis_redis::RedisStorage::<DeliverJob>::new(redis_conn);
    info!("Connected to Redis job queue");

    // Initialize fred client for distributed rate limiting
    let fred_config = fred::types::config::Config::from_url(&config.redis.url)
        .expect("Failed to parse Redis URL for rate limiter");
    let fred_client = fred::clients::Client::new(fred_config, None, None, None);
    fred_client.connect();
    fred_client.wait_for_connect().await.expect("Failed to connect fred client to Redis");
    let fred_client = Arc::new(fred_client);
    info!("Connected to Redis for distributed rate limiting");

    // Create ActivityPub delivery service
    let delivery_service: DeliveryService =
        Arc::new(RedisDeliveryService::new(redis_storage.clone()));
    let server_url = config.server.url.clone();

    // Initialize repositories
    let db = Arc::new(db);
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
    let messaging_repo = MessagingRepository::new(Arc::clone(&db));
    let clip_repo = ClipRepository::new(Arc::clone(&db));
    let antenna_repo = AntennaRepository::new(Arc::clone(&db));
    let channel_repo = ChannelRepository::new(Arc::clone(&db));
    let instance_repo = InstanceRepository::new(Arc::clone(&db));
    let word_filter_repo = WordFilterRepository::new(Arc::clone(&db));
    let scheduled_note_repo = ScheduledNoteRepository::new(Arc::clone(&db));
    let security_key_repo = SecurityKeyRepository::new(Arc::clone(&db));
    let oauth_repo = OAuthRepository::new(Arc::clone(&db));
    let webhook_repo = WebhookRepository::new(Arc::clone(&db));
    let page_repo = PageRepository::new(Arc::clone(&db));
    let gallery_repo = GalleryRepository::new(Arc::clone(&db));
    let group_repo = GroupRepository::new(Arc::clone(&db));

    // Initialize services
    let user_service = UserService::new(
        user_repo.clone(),
        user_profile_repo.clone(),
        user_keypair_repo.clone(),
        note_repo.clone(),
        &config,
    );

    // Initialize services with ActivityPub delivery support
    let note_service = if config.federation.enabled {
        NoteService::with_delivery(
            note_repo.clone(),
            user_repo.clone(),
            following_repo.clone(),
            delivery_service.clone(),
            server_url.clone(),
        )
    } else {
        NoteService::new(note_repo.clone(), user_repo.clone(), following_repo.clone())
    };

    let blocking_service = BlockingService::new(blocking_repo.clone(), following_repo.clone());

    let following_service = if config.federation.enabled {
        FollowingService::with_delivery(
            following_repo.clone(),
            follow_request_repo.clone(),
            user_repo.clone(),
            delivery_service.clone(),
            server_url.clone(),
        )
    } else {
        FollowingService::new(
            following_repo.clone(),
            follow_request_repo.clone(),
            user_repo.clone(),
        )
    };

    let reaction_service = if config.federation.enabled {
        ReactionService::with_delivery(
            reaction_repo.clone(),
            note_repo.clone(),
            user_repo.clone(),
            delivery_service.clone(),
            server_url.clone(),
        )
    } else {
        ReactionService::new(reaction_repo.clone(), note_repo.clone())
    };
    let notification_service = NotificationService::new(notification_repo);
    let muting_service = MutingService::new(muting_repo);
    let drive_service = DriveService::new(
        drive_file_repo.clone(),
        drive_folder_repo,
        config.server.url.clone(),
    );
    let poll_service = PollService::new(poll_repo, poll_vote_repo, note_repo.clone());
    let hashtag_service = misskey_core::HashtagService::new(hashtag_repo);
    let note_favorite_service = NoteFavoriteService::new(note_favorite_repo, note_repo.clone());
    let user_list_service = UserListService::new(user_list_repo, user_repo.clone());
    let moderation_service = ModerationService::new(moderation_repo, user_repo.clone());
    let emoji_service = EmojiService::new(emoji_repo);
    let announcement_service = AnnouncementService::new(announcement_repo);
    let messaging_service = MessagingService::new(messaging_repo, user_repo.clone(), blocking_repo);
    let clip_service = ClipService::new(clip_repo.clone());
    let antenna_service = AntennaService::new(antenna_repo);
    let channel_service = ChannelService::new(channel_repo);
    let instance_service = InstanceService::new(instance_repo, user_repo.clone());
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

    // Initialize OAuth service
    let oauth_service = OAuthService::new(oauth_repo);

    // Initialize Webhook service
    let webhook_service = WebhookService::new(webhook_repo);

    // Initialize Page service
    let page_service = PageService::new(page_repo);

    // Initialize Gallery service
    let gallery_service = GalleryService::new(gallery_repo);

    // Initialize Group service
    let group_service = GroupService::new(group_repo);

    // Initialize Translation service (optional, based on config)
    // For now, we set it to None. Users can configure translation in their config.
    let translation_service: Option<misskey_core::TranslationService> = None;

    // Initialize Push Notification service (optional, based on config)
    // For now, we set it to None. Users can configure VAPID keys in their config.
    let push_notification_service: Option<misskey_core::PushNotificationService> = None;

    // Initialize Account service
    let account_service = Some(AccountService::new(
        user_repo.clone(),
        user_profile_repo.clone(),
        user_keypair_repo.clone(),
        note_repo.clone(),
        following_repo.clone(),
        delivery_service.clone(),
        &config,
    ));

    // Initialize MetaSettings service
    let meta_settings_service = MetaSettingsService::new(db.clone());

    // Initialize RegistrationApproval service
    let registration_approval_service = RegistrationApprovalService::new(db.clone());

    // Initialize streaming state
    let streaming = StreamingState::new();

    // Initialize SSE broadcaster
    let sse_broadcaster = SseBroadcaster::new();

    // Initialize distributed rate limiter (uses Redis for multi-instance deployments)
    let rate_limiter = RateLimiterState::with_redis(fred_client);
    info!("Initialized distributed API rate limiter");

    // Create app state
    let state = AppState {
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
        translation_service,
        push_notification_service,
        account_service,
        group_service,
        meta_settings_service,
        registration_approval_service,
        streaming,
        sse_broadcaster,
    };

    // Create federation states
    let base_url = Url::parse(&config.server.url)?;
    let domain = base_url.host_str().unwrap_or("localhost").to_string();

    let webfinger_state = WebfingerState::new(domain.clone(), base_url.clone(), user_repo.clone());
    let nodeinfo_state = NodeInfoState::new(
        base_url.clone(),
        config.federation.instance_name.clone(),
        config
            .federation
            .instance_description
            .clone()
            .unwrap_or_default(),
        env!("CARGO_PKG_VERSION").to_string(),
        true, // open_registrations
    );
    let user_ap_state = UserApState::new(
        user_repo.clone(),
        user_keypair_repo.clone(),
        base_url.clone(),
    );

    // Create collection state for outbox/followers/following
    let collection_state = CollectionState::new(
        user_repo.clone(),
        note_repo.clone(),
        following_repo.clone(),
        drive_file_repo.clone(),
        base_url.clone(),
    );

    // Create clip collection state for ActivityPub clip collections
    let clip_collection_state = ClipCollectionState::new(
        user_repo.clone(),
        clip_repo.clone(),
        note_repo.clone(),
        drive_file_repo.clone(),
        base_url.clone(),
    );

    // Create inbox state for handling incoming ActivityPub activities
    let inbox_state = InboxState::new(
        user_repo,
        user_keypair_repo.clone(),
        note_repo,
        following_repo,
        follow_request_repo,
        reaction_repo,
        base_url.clone(),
    );

    // Build router
    let app = Router::new()
        .route("/streaming", get(streaming_handler))
        // ActivityPub / Federation endpoints
        .route(
            "/.well-known/webfinger",
            get(webfinger_handler).with_state(webfinger_state),
        )
        .route(
            "/.well-known/nodeinfo",
            get(well_known_nodeinfo).with_state(nodeinfo_state.clone()),
        )
        .route(
            "/nodeinfo/2.1",
            get(nodeinfo_2_1).with_state(nodeinfo_state),
        )
        .route(
            "/users/{id}",
            get(user_handler).with_state(user_ap_state),
        )
        .route(
            "/users/{username}/outbox",
            get(outbox_handler).with_state(collection_state.clone()),
        )
        .route(
            "/users/{username}/followers",
            get(followers_handler).with_state(collection_state.clone()),
        )
        .route(
            "/users/{username}/following",
            get(following_handler).with_state(collection_state),
        )
        // ActivityPub clip collection endpoints
        .route(
            "/users/{username}/clips",
            get(clips_list_handler).with_state(clip_collection_state.clone()),
        )
        .route(
            "/users/{username}/clips/{clip_id}",
            get(clip_handler).with_state(clip_collection_state),
        )
        // ActivityPub inbox endpoints
        .route(
            "/inbox",
            post(inbox_handler).with_state(inbox_state.clone()),
        )
        .route(
            "/users/{username}/inbox",
            post(user_inbox_handler).with_state(inbox_state),
        )
        .nest("/api", api_router())
        .layer(middleware::from_fn_with_state(
            rate_limiter,
            misskey_api::rate_limit::rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            misskey_api::middleware::auth_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Start ActivityPub delivery worker if federation is enabled
    if config.federation.enabled {
        info!("Starting ActivityPub delivery worker...");
        let worker_keypair_repo = user_keypair_repo.clone();
        let user_agent = format!("misskey-rs/{}", env!("CARGO_PKG_VERSION"));

        let deliver_ctx = DeliverContext::new(worker_keypair_repo, user_agent);

        // Spawn the worker in the background
        tokio::spawn(async move {
            let monitor = Monitor::new().register({
                WorkerBuilder::new("deliver")
                    .data(deliver_ctx)
                    .backend(redis_storage)
                    .build_fn(deliver_worker)
            });

            if let Err(e) = monitor.run().await {
                tracing::error!(error = %e, "Delivery worker failed");
            }
        });
        info!("ActivityPub delivery worker started");
    }

    // Start server with graceful shutdown
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}
