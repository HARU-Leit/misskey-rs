//! Meta endpoints.

use axum::{Json, Router, routing::post};
use serde::Serialize;

use crate::middleware::AppState;

/// Server metadata response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaResponse {
    pub maintainer_name: Option<String>,
    pub maintainer_email: Option<String>,
    pub version: String,
    pub name: String,
    pub short_name: String,
    pub uri: String,
    pub description: Option<String>,
    pub langs: Vec<String>,
    pub disable_registration: bool,
    pub email_required_for_signup: bool,
}

/// Get server metadata.
async fn meta() -> Json<MetaResponse> {
    Json(MetaResponse {
        maintainer_name: None,
        maintainer_email: None,
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "misskey-rs".to_string(),
        short_name: "misskey-rs".to_string(),
        uri: "localhost".to_string(),
        description: Some("A Misskey server implemented in Rust".to_string()),
        langs: vec!["ja".to_string(), "en".to_string()],
        disable_registration: false,
        email_required_for_signup: false,
    })
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", post(meta))
}
