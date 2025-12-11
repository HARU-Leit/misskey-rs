//! `NodeInfo` handler for instance discovery.

#![allow(clippy::expect_used)] // URL joins with known-valid paths cannot fail

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use misskey_db::repositories::{NoteRepository, UserRepository};
use serde::Serialize;
use std::sync::Arc;
use url::Url;

/// `NodeInfo` well-known response.
#[derive(Debug, Serialize)]
pub struct NodeInfoWellKnown {
    pub links: Vec<NodeInfoLink>,
}

/// `NodeInfo` link.
#[derive(Debug, Serialize)]
pub struct NodeInfoLink {
    pub rel: String,
    pub href: String,
}

/// `NodeInfo` 2.1 response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    pub version: String,
    pub software: NodeInfoSoftware,
    pub protocols: Vec<String>,
    pub usage: NodeInfoUsage,
    pub open_registrations: bool,
    pub metadata: NodeInfoMetadata,
}

/// `NodeInfo` software information.
#[derive(Debug, Serialize)]
pub struct NodeInfoSoftware {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
}

/// `NodeInfo` usage statistics.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfoUsage {
    pub users: NodeInfoUsers,
    pub local_posts: u64,
}

/// `NodeInfo` user statistics.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfoUsers {
    pub total: u64,
    pub active_month: u64,
    pub active_halfyear: u64,
}

/// `NodeInfo` metadata.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfoMetadata {
    pub node_name: String,
    pub node_description: String,
    pub maintainer: NodeInfoMaintainer,
    pub theme_color: String,
}

/// `NodeInfo` maintainer.
#[derive(Debug, Serialize)]
pub struct NodeInfoMaintainer {
    pub name: Option<String>,
    pub email: Option<String>,
}

/// State for `NodeInfo` handlers.
#[derive(Clone)]
pub struct NodeInfoState {
    pub base_url: Url,
    pub instance_name: String,
    pub instance_description: String,
    pub version: String,
    pub open_registrations: bool,
    pub user_repo: Option<Arc<UserRepository>>,
    pub note_repo: Option<Arc<NoteRepository>>,
}

impl NodeInfoState {
    /// Create new `NodeInfo` state.
    #[must_use]
    pub const fn new(
        base_url: Url,
        instance_name: String,
        instance_description: String,
        version: String,
        open_registrations: bool,
    ) -> Self {
        Self {
            base_url,
            instance_name,
            instance_description,
            version,
            open_registrations,
            user_repo: None,
            note_repo: None,
        }
    }

    /// Create new `NodeInfo` state with repositories for statistics.
    #[must_use]
    pub fn with_repos(
        base_url: Url,
        instance_name: String,
        instance_description: String,
        version: String,
        open_registrations: bool,
        user_repo: UserRepository,
        note_repo: NoteRepository,
    ) -> Self {
        Self {
            base_url,
            instance_name,
            instance_description,
            version,
            open_registrations,
            user_repo: Some(Arc::new(user_repo)),
            note_repo: Some(Arc::new(note_repo)),
        }
    }
}

/// Handle /.well-known/nodeinfo
pub async fn well_known_nodeinfo(State(state): State<NodeInfoState>) -> impl IntoResponse {
    let nodeinfo_url = state.base_url.join("/nodeinfo/2.1").expect("valid URL");

    let response = NodeInfoWellKnown {
        links: vec![NodeInfoLink {
            rel: "http://nodeinfo.diaspora.software/ns/schema/2.1".to_string(),
            href: nodeinfo_url.to_string(),
        }],
    };

    (
        StatusCode::OK,
        [("Content-Type", "application/json")],
        Json(response),
    )
}

/// Handle /nodeinfo/2.1
pub async fn nodeinfo_2_1(State(state): State<NodeInfoState>) -> impl IntoResponse {
    // Get actual statistics from database if repositories are available
    let (total_users, active_month, active_halfyear, local_posts) = get_statistics(&state).await;

    let response = NodeInfo {
        version: "2.1".to_string(),
        software: NodeInfoSoftware {
            name: "misskey-rs".to_string(),
            version: state.version.clone(),
            repository: Some("https://github.com/example/misskey-rs".to_string()),
            homepage: Some(state.base_url.to_string()),
        },
        protocols: vec!["activitypub".to_string()],
        usage: NodeInfoUsage {
            users: NodeInfoUsers {
                total: total_users,
                active_month,
                active_halfyear,
            },
            local_posts,
        },
        open_registrations: state.open_registrations,
        metadata: NodeInfoMetadata {
            node_name: state.instance_name.clone(),
            node_description: state.instance_description,
            maintainer: NodeInfoMaintainer {
                name: None,
                email: None,
            },
            theme_color: "#86b300".to_string(),
        },
    };

    (
        StatusCode::OK,
        [(
            "Content-Type",
            "application/json; profile=\"http://nodeinfo.diaspora.software/ns/schema/2.1#\"",
        )],
        Json(response),
    )
}

/// Get statistics from database repositories.
async fn get_statistics(state: &NodeInfoState) -> (u64, u64, u64, u64) {
    let mut total_users = 0u64;
    let mut active_month = 0u64;
    let mut active_halfyear = 0u64;
    let mut local_posts = 0u64;

    // Get user statistics
    if let Some(ref user_repo) = state.user_repo {
        if let Ok(count) = user_repo.count_local_users().await {
            total_users = count;
        }
        if let Ok(count) = user_repo.count_active_local_users_month().await {
            active_month = count;
        }
        if let Ok(count) = user_repo.count_active_local_users_halfyear().await {
            active_halfyear = count;
        }
    }

    // Get note statistics
    if let Some(ref note_repo) = state.note_repo
        && let Ok(count) = note_repo.count_local_notes().await
    {
        local_posts = count;
    }

    (total_users, active_month, active_halfyear, local_posts)
}
