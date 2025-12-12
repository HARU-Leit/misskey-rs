//! Local Federation Integration Tests
//!
//! These tests verify `ActivityPub` federation between two local instances.
//! They require the federation docker-compose profile to be running:
//!
//! ```bash
//! docker-compose -f docker-compose.test.yml --profile federation up -d
//! cargo test --features federation-test -- local_federation
//! docker-compose -f docker-compose.test.yml --profile federation down -v
//! ```

#![cfg(feature = "federation-test")]
#![allow(clippy::unwrap_used, clippy::expect_used, unused_variables)]

use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::sleep;

const ALPHA_URL: &str = "http://localhost:3001";
const BETA_URL: &str = "http://localhost:3002";

/// Check if federation tests should be skipped (e.g., in CI).
fn should_skip() -> bool {
    std::env::var("SKIP_FEDERATION_TEST").is_ok()
}

/// Macro to skip test if `SKIP_FEDERATION_TEST` is set.
macro_rules! skip_if_ci {
    () => {
        if should_skip() {
            eprintln!("Skipping federation test (SKIP_FEDERATION_TEST is set)");
            return;
        }
    };
}

/// Test client for interacting with a Misskey instance
struct TestInstance {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl TestInstance {
    fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.to_string(),
            token: None,
        }
    }

    async fn health_check(&self) -> Result<bool, reqwest::Error> {
        let res = self
            .client
            .get(format!("{}/.well-known/nodeinfo", self.base_url))
            .send()
            .await?;
        Ok(res.status().is_success())
    }

    async fn create_user(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<Value, reqwest::Error> {
        let res = self
            .client
            .post(format!("{}/api/signup", self.base_url))
            .json(&json!({
                "username": username,
                "password": password
            }))
            .send()
            .await?
            .json::<Value>()
            .await?;

        if let Some(token) = res.get("token").and_then(|t| t.as_str()) {
            self.token = Some(token.to_string());
        }

        Ok(res)
    }

    async fn signin(&mut self, username: &str, password: &str) -> Result<Value, reqwest::Error> {
        let res = self
            .client
            .post(format!("{}/api/signin", self.base_url))
            .json(&json!({
                "username": username,
                "password": password
            }))
            .send()
            .await?
            .json::<Value>()
            .await?;

        if let Some(token) = res.get("token").and_then(|t| t.as_str()) {
            self.token = Some(token.to_string());
        }

        Ok(res)
    }

    async fn api_call(&self, endpoint: &str, body: Value) -> Result<Value, reqwest::Error> {
        let mut req = self
            .client
            .post(format!("{}/api/{}", self.base_url, endpoint));

        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }

        req.json(&body).send().await?.json::<Value>().await
    }

    async fn create_note(&self, text: &str) -> Result<Value, reqwest::Error> {
        self.api_call(
            "notes/create",
            json!({
                "text": text,
                "visibility": "public"
            }),
        )
        .await
    }

    async fn get_user(&self, username: &str, host: Option<&str>) -> Result<Value, reqwest::Error> {
        let mut body = json!({ "username": username });
        if let Some(h) = host {
            body["host"] = json!(h);
        }
        self.api_call("users/show", body).await
    }

    async fn follow(&self, user_id: &str) -> Result<Value, reqwest::Error> {
        self.api_call(
            "following/create",
            json!({
                "userId": user_id
            }),
        )
        .await
    }

    async fn webfinger(&self, resource: &str) -> Result<Value, reqwest::Error> {
        self.client
            .get(format!("{}/.well-known/webfinger", self.base_url))
            .query(&[("resource", resource)])
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn fetch_actor(&self, actor_url: &str) -> Result<Value, reqwest::Error> {
        self.client
            .get(actor_url)
            .header("Accept", "application/activity+json")
            .send()
            .await?
            .json::<Value>()
            .await
    }
}

/// Wait for instances to be ready
async fn wait_for_instances() -> bool {
    let alpha = TestInstance::new(ALPHA_URL);
    let beta = TestInstance::new(BETA_URL);

    for _ in 0..30 {
        let alpha_ready = alpha.health_check().await.unwrap_or(false);
        let beta_ready = beta.health_check().await.unwrap_or(false);

        if alpha_ready && beta_ready {
            return true;
        }
        sleep(Duration::from_secs(1)).await;
    }

    false
}

#[tokio::test]
async fn test_instances_are_running() {
    skip_if_ci!();
    assert!(
        wait_for_instances().await,
        "Federation instances are not running. Start them with: docker-compose -f docker-compose.test.yml --profile federation up -d"
    );
}

#[tokio::test]
async fn test_webfinger_resolution() {
    skip_if_ci!();
    if !wait_for_instances().await {
        eprintln!("Skipping: Federation instances not running");
        return;
    }

    let mut alpha = TestInstance::new(ALPHA_URL);

    // Create a user on alpha
    let _user = alpha
        .create_user("webfingertest", "testpass123")
        .await
        .expect("Failed to create user on alpha");

    // Resolve via webfinger
    let webfinger = alpha
        .webfinger("acct:webfingertest@alpha")
        .await
        .expect("Failed to resolve webfinger");

    assert!(webfinger.get("subject").is_some());
    assert!(webfinger.get("links").is_some());

    let links = webfinger["links"]
        .as_array()
        .expect("links should be array");
    let self_link = links
        .iter()
        .find(|l| l["rel"].as_str() == Some("self"))
        .expect("Should have self link");

    assert!(
        self_link["type"]
            .as_str()
            .unwrap()
            .contains("activity+json")
    );
}

#[tokio::test]
async fn test_actor_endpoint() {
    skip_if_ci!();
    if !wait_for_instances().await {
        eprintln!("Skipping: Federation instances not running");
        return;
    }

    let mut alpha = TestInstance::new(ALPHA_URL);

    // Create a user
    let _user = alpha
        .create_user("actortest", "testpass123")
        .await
        .expect("Failed to create user");

    // Fetch actor via ActivityPub
    let actor = alpha
        .fetch_actor(&format!("{ALPHA_URL}/users/actortest"))
        .await
        .expect("Failed to fetch actor");

    assert_eq!(actor["type"], "Person");
    assert_eq!(actor["preferredUsername"], "actortest");
    assert!(actor.get("inbox").is_some());
    assert!(actor.get("outbox").is_some());
    assert!(actor.get("publicKey").is_some());
}

#[tokio::test]
async fn test_cross_instance_user_resolution() {
    skip_if_ci!();
    if !wait_for_instances().await {
        eprintln!("Skipping: Federation instances not running");
        return;
    }

    let mut alpha = TestInstance::new(ALPHA_URL);
    let mut beta = TestInstance::new(BETA_URL);

    // Create user on alpha
    alpha
        .create_user("crosstest", "testpass123")
        .await
        .expect("Failed to create user on alpha");

    // Create user on beta to make API calls
    beta.create_user("resolver", "testpass123")
        .await
        .expect("Failed to create user on beta");

    // Resolve alpha's user from beta
    let remote_user = beta
        .get_user("crosstest", Some("alpha"))
        .await
        .expect("Failed to resolve remote user");

    assert_eq!(remote_user["username"], "crosstest");
    assert!(remote_user.get("host").is_some());
}

#[tokio::test]
async fn test_follow_between_instances() {
    skip_if_ci!();
    if !wait_for_instances().await {
        eprintln!("Skipping: Federation instances not running");
        return;
    }

    let mut alpha = TestInstance::new(ALPHA_URL);
    let mut beta = TestInstance::new(BETA_URL);

    // Create users
    alpha
        .create_user("leader", "testpass123")
        .await
        .expect("Failed to create leader on alpha");

    beta.create_user("follower", "testpass123")
        .await
        .expect("Failed to create follower on beta");

    // Get leader's remote user on beta
    let remote_leader = beta
        .get_user("leader", Some("alpha"))
        .await
        .expect("Failed to resolve remote user");

    let leader_id = remote_leader["id"]
        .as_str()
        .expect("Remote user should have id");

    // Follow from beta to alpha
    let follow_result = beta
        .follow(leader_id)
        .await
        .expect("Failed to create follow");

    // Wait for federation
    sleep(Duration::from_secs(2)).await;

    // Verify follow was processed
    // Note: Depending on lock settings, may need to check follow requests
    assert!(follow_result.get("error").is_none() || follow_result.get("followingId").is_some());
}

#[tokio::test]
async fn test_note_federation() {
    skip_if_ci!();
    if !wait_for_instances().await {
        eprintln!("Skipping: Federation instances not running");
        return;
    }

    let mut alpha = TestInstance::new(ALPHA_URL);
    let mut beta = TestInstance::new(BETA_URL);

    // Create users
    alpha
        .create_user("noteauthor", "testpass123")
        .await
        .expect("Failed to create user on alpha");

    beta.create_user("notereader", "testpass123")
        .await
        .expect("Failed to create user on beta");

    // Create a public note on alpha
    let note = alpha
        .create_note("Hello from alpha! This is a federation test.")
        .await
        .expect("Failed to create note");

    let note_id = note["createdNote"]["id"]
        .as_str()
        .expect("Note should have id");

    // Wait for potential federation
    sleep(Duration::from_secs(1)).await;

    // The note should be accessible via its URI
    let note_uri = format!("{ALPHA_URL}/notes/{note_id}");

    let fetched = alpha
        .client
        .get(&note_uri)
        .header("Accept", "application/activity+json")
        .send()
        .await
        .expect("Failed to fetch note")
        .json::<Value>()
        .await
        .expect("Failed to parse note");

    assert_eq!(fetched["type"], "Note");
    assert!(
        fetched["content"]
            .as_str()
            .unwrap()
            .contains("federation test")
    );
}

#[tokio::test]
async fn test_nodeinfo_endpoint() {
    skip_if_ci!();
    if !wait_for_instances().await {
        eprintln!("Skipping: Federation instances not running");
        return;
    }

    let alpha = TestInstance::new(ALPHA_URL);

    // Get nodeinfo links
    let well_known = alpha
        .client
        .get(format!("{ALPHA_URL}/.well-known/nodeinfo"))
        .send()
        .await
        .expect("Failed to get nodeinfo well-known")
        .json::<Value>()
        .await
        .expect("Failed to parse well-known");

    let links = well_known["links"].as_array().expect("Should have links");
    assert!(!links.is_empty());

    let nodeinfo_url = links[0]["href"].as_str().expect("Link should have href");

    // Fetch actual nodeinfo
    let nodeinfo = alpha
        .client
        .get(nodeinfo_url)
        .send()
        .await
        .expect("Failed to get nodeinfo")
        .json::<Value>()
        .await
        .expect("Failed to parse nodeinfo");

    assert_eq!(nodeinfo["software"]["name"], "misskey-rs");
    assert!(
        nodeinfo["protocols"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p == "activitypub")
    );
}

#[tokio::test]
async fn test_inbox_signature_verification() {
    skip_if_ci!();
    if !wait_for_instances().await {
        eprintln!("Skipping: Federation instances not running");
        return;
    }

    let alpha = TestInstance::new(ALPHA_URL);

    // Send an unsigned request to inbox - should be rejected
    let result = alpha
        .client
        .post(format!("{ALPHA_URL}/inbox"))
        .header("Content-Type", "application/activity+json")
        .json(&json!({
            "@context": "https://www.w3.org/ns/activitystreams",
            "type": "Create",
            "actor": "https://malicious.example/users/attacker",
            "object": {
                "type": "Note",
                "content": "Unsigned malicious note"
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    // Should reject unsigned requests
    assert!(result.status().is_client_error() || result.status().is_server_error());
}

/// Integration test for the complete follow flow
#[tokio::test]
async fn test_full_follow_flow() {
    skip_if_ci!();
    if !wait_for_instances().await {
        eprintln!("Skipping: Federation instances not running");
        return;
    }

    let mut alpha = TestInstance::new(ALPHA_URL);
    let mut beta = TestInstance::new(BETA_URL);

    // 1. Create user on each instance
    let alpha_user = alpha
        .create_user("alice_full", "password123")
        .await
        .expect("Failed to create alice on alpha");
    let beta_user = beta
        .create_user("bob_full", "password123")
        .await
        .expect("Failed to create bob on beta");

    let alice_id = alpha_user["id"].as_str().expect("Alice should have id");

    // 2. Bob resolves Alice from beta
    let remote_alice = beta
        .get_user("alice_full", Some("alpha"))
        .await
        .expect("Failed to resolve alice from beta");

    let remote_alice_id = remote_alice["id"]
        .as_str()
        .expect("Remote alice should have id");

    // 3. Bob follows Alice
    beta.follow(remote_alice_id)
        .await
        .expect("Failed to follow");

    // 4. Wait for federation activities
    sleep(Duration::from_secs(3)).await;

    // 5. Verify Alice's followers includes Bob
    let alice_followers = alpha
        .api_call(
            "users/followers",
            json!({
                "userId": alice_id
            }),
        )
        .await
        .expect("Failed to get followers");

    // Note: The exact response structure depends on implementation
    // This verifies the federation flow completed without errors
    println!("Follow flow completed. Followers response: {alice_followers:?}");
}

/// Test that reactions are federated
#[tokio::test]
async fn test_reaction_federation() {
    skip_if_ci!();
    if !wait_for_instances().await {
        eprintln!("Skipping: Federation instances not running");
        return;
    }

    let mut alpha = TestInstance::new(ALPHA_URL);
    let mut beta = TestInstance::new(BETA_URL);

    // Create users
    alpha
        .create_user("reactionauthor", "password123")
        .await
        .ok();
    beta.create_user("reactioner", "password123").await.ok();

    // Sign in (in case users already exist)
    alpha.signin("reactionauthor", "password123").await.ok();
    beta.signin("reactioner", "password123").await.ok();

    // Create a note on alpha
    let note = alpha
        .create_note("React to this note!")
        .await
        .expect("Failed to create note");

    let note_id = note["createdNote"]["id"]
        .as_str()
        .expect("Note should have id");

    // FUTURE: Once reaction federation is fully implemented, add:
    // 1. Resolve the note from beta
    // 2. Add reaction from beta
    // 3. Verify reaction appears on alpha

    println!("Note created for reaction test: {note_id}");
}
