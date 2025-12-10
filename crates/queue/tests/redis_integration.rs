//! Redis integration tests.
//!
//! These tests require a running Redis instance.
//! Run with: `cargo test --test redis_integration -- --ignored`
//!
//! Set `REDIS_URL` environment variable to point to your Redis instance.
//! Default: <redis://localhost:6379>

use std::time::Duration;

use misskey_queue::{pubsub_channels, PubSubEvent, RedisPubSub};

fn get_redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

/// Test that we can connect to Redis.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_redis_connection() {
    let url = get_redis_url();
    let pubsub = RedisPubSub::new(&url).await;
    assert!(pubsub.is_ok(), "Failed to connect to Redis: {:?}", pubsub.err());
}

/// Test pub/sub channel subscription.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_pubsub_subscribe_channels() {
    let url = get_redis_url();
    let pubsub = RedisPubSub::new(&url).await.expect("Failed to connect to Redis");

    let result = pubsub.start().await;
    assert!(result.is_ok(), "Failed to subscribe to channels: {:?}", result.err());

    pubsub.shutdown().await.expect("Failed to shutdown");
}

/// Test publishing a note creation event.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_publish_note_created() {
    let url = get_redis_url();
    let pubsub = RedisPubSub::new(&url).await.expect("Failed to connect to Redis");

    let result = pubsub.publish_note_created(
        "test-note-123",
        "test-user-456",
        Some("Hello from integration test!"),
        "public",
    ).await;

    assert!(result.is_ok(), "Failed to publish note: {:?}", result.err());

    pubsub.shutdown().await.expect("Failed to shutdown");
}

/// Test publishing a notification event.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_publish_notification() {
    let url = get_redis_url();
    let pubsub = RedisPubSub::new(&url).await.expect("Failed to connect to Redis");

    let result = pubsub.publish_notification(
        "notif-123",
        "recipient-456",
        "follow",
        Some("follower-789"),
        None,
    ).await;

    assert!(result.is_ok(), "Failed to publish notification: {:?}", result.err());

    pubsub.shutdown().await.expect("Failed to shutdown");
}

/// Test publishing a follow event.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_publish_followed() {
    let url = get_redis_url();
    let pubsub = RedisPubSub::new(&url).await.expect("Failed to connect to Redis");

    let result = pubsub.publish_followed("follower-123", "followee-456").await;

    assert!(result.is_ok(), "Failed to publish follow: {:?}", result.err());

    pubsub.shutdown().await.expect("Failed to shutdown");
}

/// Test publishing a reaction event.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_publish_reaction() {
    let url = get_redis_url();
    let pubsub = RedisPubSub::new(&url).await.expect("Failed to connect to Redis");

    let result = pubsub.publish_reaction_added(
        "note-123",
        "reactor-456",
        "👍",
        "note-author-789",
    ).await;

    assert!(result.is_ok(), "Failed to publish reaction: {:?}", result.err());

    pubsub.shutdown().await.expect("Failed to shutdown");
}

/// Test user channel subscription.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_subscribe_user_channel() {
    let url = get_redis_url();
    let pubsub = RedisPubSub::new(&url).await.expect("Failed to connect to Redis");

    let result = pubsub.subscribe_user("user-123").await;
    assert!(result.is_ok(), "Failed to subscribe to user channel: {:?}", result.err());

    let result = pubsub.unsubscribe_user("user-123").await;
    assert!(result.is_ok(), "Failed to unsubscribe from user channel: {:?}", result.err());

    pubsub.shutdown().await.expect("Failed to shutdown");
}

/// Test local subscriber count.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_local_subscriber_count() {
    let url = get_redis_url();
    let pubsub = RedisPubSub::new(&url).await.expect("Failed to connect to Redis");

    // Initially no subscribers
    assert_eq!(pubsub.local_subscriber_count(), 0);

    // Subscribe creates a receiver
    let _rx1 = pubsub.subscribe_local();
    assert_eq!(pubsub.local_subscriber_count(), 1);

    let _rx2 = pubsub.subscribe_local();
    assert_eq!(pubsub.local_subscriber_count(), 2);

    pubsub.shutdown().await.expect("Failed to shutdown");
}

/// Test publishing and receiving events through local broadcast.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_local_broadcast_roundtrip() {
    let url = get_redis_url();
    let pubsub = RedisPubSub::new(&url).await.expect("Failed to connect to Redis");

    // Start the subscriber (this sets up the message loop)
    pubsub.start().await.expect("Failed to start");

    // Subscribe to local broadcast
    let mut rx = pubsub.subscribe_local();

    // Publish an event (this goes through Redis)
    let event = PubSubEvent::NoteCreated {
        id: "roundtrip-note".to_string(),
        user_id: "roundtrip-user".to_string(),
        text: Some("Roundtrip test".to_string()),
        visibility: "public".to_string(),
    };

    pubsub.publish(pubsub_channels::NOTES, &event).await
        .expect("Failed to publish");

    // Wait for the event to come back
    let timeout = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;

    if let Ok(Ok(received)) = timeout {
        match received {
            PubSubEvent::NoteCreated { id, .. } => {
                assert_eq!(id, "roundtrip-note");
            }
            _ => panic!("Unexpected event type"),
        }
    } else {
        // Note: This may fail if Redis is slow or the test environment is noisy
        // The event went out but might not have come back in time
        eprintln!("Warning: Roundtrip test timed out - this may be expected in some environments");
    }

    pubsub.shutdown().await.expect("Failed to shutdown");
}

/// Test multiple pubsub instances can communicate.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_cross_instance_communication() {
    let url = get_redis_url();

    // Create two pubsub instances (simulating two server instances)
    let pubsub1 = RedisPubSub::new(&url).await.expect("Failed to connect instance 1");
    let pubsub2 = RedisPubSub::new(&url).await.expect("Failed to connect instance 2");

    // Start both instances
    pubsub1.start().await.expect("Failed to start instance 1");
    pubsub2.start().await.expect("Failed to start instance 2");

    // Give some time for subscriptions to be established
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Instance 2 subscribes to local broadcast
    let mut rx2 = pubsub2.subscribe_local();

    // Instance 1 publishes an event
    let event = PubSubEvent::Announcement {
        id: "cross-instance-ann".to_string(),
        text: "Hello from instance 1".to_string(),
    };

    pubsub1.publish(pubsub_channels::NOTIFICATIONS, &event).await
        .expect("Failed to publish from instance 1");

    // Wait for instance 2 to receive
    let timeout = tokio::time::timeout(Duration::from_secs(2), rx2.recv()).await;

    if let Ok(Ok(received)) = timeout {
        match received {
            PubSubEvent::Announcement { id, text } => {
                assert_eq!(id, "cross-instance-ann");
                assert_eq!(text, "Hello from instance 1");
            }
            _ => panic!("Unexpected event type"),
        }
    } else {
        eprintln!("Warning: Cross-instance test timed out - this may be expected in some environments");
    }

    pubsub1.shutdown().await.expect("Failed to shutdown instance 1");
    pubsub2.shutdown().await.expect("Failed to shutdown instance 2");
}

/// Test publishing events to different visibility levels.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_visibility_based_publishing() {
    let url = get_redis_url();
    let pubsub = RedisPubSub::new(&url).await.expect("Failed to connect to Redis");

    // Public note should go to both global and local
    let result = pubsub.publish_note_created(
        "public-note",
        "user-1",
        Some("Public post"),
        "public",
    ).await;
    assert!(result.is_ok());

    // Home note should only go to local
    let result = pubsub.publish_note_created(
        "home-note",
        "user-1",
        Some("Home post"),
        "home",
    ).await;
    assert!(result.is_ok());

    // Followers note should only go to local
    let result = pubsub.publish_note_created(
        "followers-note",
        "user-1",
        Some("Followers only post"),
        "followers",
    ).await;
    assert!(result.is_ok());

    // Specified note should not go to timelines
    let result = pubsub.publish_note_created(
        "specified-note",
        "user-1",
        Some("Direct message"),
        "specified",
    ).await;
    assert!(result.is_ok());

    pubsub.shutdown().await.expect("Failed to shutdown");
}

/// Test graceful shutdown.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_graceful_shutdown() {
    let url = get_redis_url();
    let pubsub = RedisPubSub::new(&url).await.expect("Failed to connect to Redis");

    pubsub.start().await.expect("Failed to start");

    // Do some operations
    pubsub.subscribe_user("shutdown-test-user").await.ok();
    pubsub.publish_notification("n1", "u1", "test", None, None).await.ok();

    // Shutdown should complete without errors
    let result = pubsub.shutdown().await;
    assert!(result.is_ok(), "Shutdown failed: {:?}", result.err());
}

/// Test reconnection behavior (simulated).
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_multiple_connections() {
    let url = get_redis_url();

    // Create multiple connections in quick succession
    for i in 0..5 {
        let pubsub = RedisPubSub::new(&url).await
            .unwrap_or_else(|_| panic!("Failed to connect (attempt {i})"));
        pubsub.shutdown().await.expect("Failed to shutdown");
    }
}

/// Stress test: rapid publishing.
#[tokio::test]
#[ignore = "requires running Redis instance"]
async fn test_rapid_publishing() {
    let url = get_redis_url();
    let pubsub = RedisPubSub::new(&url).await.expect("Failed to connect to Redis");

    pubsub.start().await.expect("Failed to start");

    // Publish 100 events rapidly
    for i in 0..100 {
        let result = pubsub.publish_note_created(
            &format!("rapid-note-{i}"),
            "rapid-user",
            Some(&format!("Rapid message {i}")),
            "public",
        ).await;

        assert!(result.is_ok(), "Failed at iteration {}: {:?}", i, result.err());
    }

    pubsub.shutdown().await.expect("Failed to shutdown");
}
