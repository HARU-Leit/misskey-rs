//! Queue integration tests.
//!
//! These tests verify the queue components work correctly together.

use std::time::Duration;

use misskey_queue::{
    InstanceRateLimiter, PubSubEvent, RateLimitConfig, RateLimitResult, SchedulerConfig,
    SchedulerState,
};

#[tokio::test]
async fn test_rate_limiter_multiple_instances_isolation() {
    let config = RateLimitConfig {
        max_requests: 10,
        window: Duration::from_secs(60),
        cooldown: Duration::from_secs(300),
    };
    let limiter = InstanceRateLimiter::new(config);

    // Instance A makes requests
    for _ in 0..5 {
        assert_eq!(
            limiter.check("instance-a.example.com").await,
            RateLimitResult::Allowed
        );
    }

    // Instance B should have full quota
    for _ in 0..10 {
        assert_eq!(
            limiter.check("instance-b.example.com").await,
            RateLimitResult::Allowed
        );
    }

    // Instance A should still have quota
    assert_eq!(
        limiter.check("instance-a.example.com").await,
        RateLimitResult::Allowed
    );

    // Instance B should be in cooldown
    match limiter.check("instance-b.example.com").await {
        RateLimitResult::Cooldown { .. } => {}
        other => panic!("Expected Cooldown, got {other:?}"),
    }
}

#[tokio::test]
async fn test_rate_limiter_concurrent_access() {
    let config = RateLimitConfig {
        max_requests: 100,
        window: Duration::from_secs(60),
        cooldown: Duration::from_secs(10),
    };
    let limiter = InstanceRateLimiter::new(config);

    // Spawn multiple tasks accessing the same instance
    let handles: Vec<_> = (0..50)
        .map(|_| {
            let limiter = limiter.clone();
            tokio::spawn(async move { limiter.check("concurrent.example.com").await })
        })
        .collect();

    let mut allowed_count = 0;
    for handle in handles {
        if matches!(handle.await, Ok(RateLimitResult::Allowed)) {
            allowed_count += 1;
        }
    }

    // All 50 should be allowed (limit is 100)
    assert_eq!(allowed_count, 50);
    assert_eq!(limiter.instance_count().await, 1);
}

#[tokio::test]
async fn test_rate_limiter_cleanup() {
    let config = RateLimitConfig {
        max_requests: 5,
        window: Duration::from_millis(10), // Very short window
        cooldown: Duration::from_millis(10),
    };
    let limiter = InstanceRateLimiter::new(config);

    // Create entries for multiple instances
    limiter.check("cleanup-1.example.com").await;
    limiter.check("cleanup-2.example.com").await;
    limiter.check("cleanup-3.example.com").await;

    assert_eq!(limiter.instance_count().await, 3);

    // Wait for windows to expire
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Cleanup should remove expired entries
    limiter.cleanup().await;

    assert_eq!(limiter.instance_count().await, 0);
}

#[test]
fn test_scheduler_state_default() {
    let state = SchedulerState::default();

    // All last run times should be None initially
    assert!(state.last_mute_cleanup.is_none());
    assert!(state.last_health_check.is_none());
    assert!(state.last_chart_aggregation.is_none());
    assert!(state.last_note_cleanup.is_none());
}

#[test]
fn test_scheduler_config_intervals() {
    let config = SchedulerConfig::default();

    // Verify default intervals are reasonable
    assert!(config.mute_cleanup_interval >= Duration::from_secs(60));
    assert!(config.health_check_interval >= Duration::from_secs(30));
    assert!(config.chart_aggregation_interval >= Duration::from_secs(60));
    assert!(!config.enable_note_cleanup);
    assert_eq!(config.note_retention_days, 365);
}

#[test]
fn test_pubsub_event_roundtrip() {
    // Test that all event types can be serialized and deserialized
    let events = vec![
        PubSubEvent::NoteCreated {
            id: "note1".to_string(),
            user_id: "user1".to_string(),
            text: Some("Hello world".to_string()),
            visibility: "public".to_string(),
        },
        PubSubEvent::NoteDeleted {
            id: "note2".to_string(),
            user_id: "user1".to_string(),
        },
        PubSubEvent::Notification {
            id: "notif1".to_string(),
            user_id: "user1".to_string(),
            notification_type: "follow".to_string(),
            source_user_id: Some("user2".to_string()),
            note_id: None,
        },
        PubSubEvent::Followed {
            follower_id: "user2".to_string(),
            followee_id: "user1".to_string(),
        },
        PubSubEvent::Unfollowed {
            follower_id: "user2".to_string(),
            followee_id: "user1".to_string(),
        },
        PubSubEvent::ReactionAdded {
            note_id: "note1".to_string(),
            user_id: "user2".to_string(),
            reaction: "like".to_string(),
        },
        PubSubEvent::ReactionRemoved {
            note_id: "note1".to_string(),
            user_id: "user2".to_string(),
            reaction: "like".to_string(),
        },
        PubSubEvent::UserUpdated {
            user_id: "user1".to_string(),
        },
        PubSubEvent::Announcement {
            id: "ann1".to_string(),
            text: "Welcome!".to_string(),
        },
    ];

    for event in events {
        let json = serde_json::to_string(&event).expect("Serialization failed");
        let parsed: PubSubEvent = serde_json::from_str(&json).expect("Deserialization failed");

        // Verify roundtrip preserves data
        let json2 = serde_json::to_string(&parsed).expect("Re-serialization failed");
        assert_eq!(json, json2);
    }
}

#[test]
fn test_pubsub_event_has_type_field() {
    let event = PubSubEvent::NoteCreated {
        id: "test".to_string(),
        user_id: "user".to_string(),
        text: None,
        visibility: "public".to_string(),
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"type\":\"noteCreated\""));
}
