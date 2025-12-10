//! Admin/Moderation endpoints.

use axum::{extract::State, routing::post, Json, Router};
use misskey_common::AppResult;
use misskey_core::{CreateReportInput, CreateSuspensionInput, ReportStatus, ResolveReportInput, UpdateInstanceInput};
use misskey_db::entities::{abuse_report, instance, user_suspension};
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

/// Abuse report response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportResponse {
    pub id: String,
    pub reporter_id: String,
    pub target_user_id: String,
    pub target_note_id: Option<String>,
    pub comment: String,
    pub status: String,
    pub assignee_id: Option<String>,
    pub resolution_comment: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

impl From<abuse_report::Model> for ReportResponse {
    fn from(report: abuse_report::Model) -> Self {
        Self {
            id: report.id,
            reporter_id: report.reporter_id,
            target_user_id: report.target_user_id,
            target_note_id: report.target_note_id,
            comment: report.comment,
            status: match report.status {
                ReportStatus::Pending => "pending".to_string(),
                ReportStatus::Resolved => "resolved".to_string(),
                ReportStatus::Rejected => "rejected".to_string(),
            },
            assignee_id: report.assignee_id,
            resolution_comment: report.resolution_comment,
            created_at: report.created_at.to_rfc3339(),
            resolved_at: report.resolved_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// Suspension response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuspensionResponse {
    pub id: String,
    pub user_id: String,
    pub moderator_id: String,
    pub reason: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub lifted_at: Option<String>,
    pub lifted_by: Option<String>,
}

impl From<user_suspension::Model> for SuspensionResponse {
    fn from(suspension: user_suspension::Model) -> Self {
        Self {
            id: suspension.id,
            user_id: suspension.user_id,
            moderator_id: suspension.moderator_id,
            reason: suspension.reason,
            created_at: suspension.created_at.to_rfc3339(),
            expires_at: suspension.expires_at.map(|t| t.to_rfc3339()),
            lifted_at: suspension.lifted_at.map(|t| t.to_rfc3339()),
            lifted_by: suspension.lifted_by,
        }
    }
}

/// Create report request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReportRequest {
    pub user_id: String,
    pub note_id: Option<String>,
    pub comment: String,
}

/// Get reports request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetReportsRequest {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

const fn default_limit() -> u64 {
    10
}

/// Resolve report request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveReportRequest {
    pub report_id: String,
    pub resolution: String,
    pub comment: Option<String>,
}

/// Suspend user request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuspendUserRequest {
    pub user_id: String,
    pub reason: String,
    /// Duration in seconds, null for permanent.
    pub duration: Option<i64>,
}

/// Unsuspend user request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsuspendUserRequest {
    pub user_id: String,
}

/// Get report request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetReportRequest {
    pub report_id: String,
}

/// Admin queue stats response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminStatsResponse {
    pub pending_reports: u64,
    pub active_suspensions: u64,
}

// ==================== Instance Types ====================

/// Instance response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceResponse {
    pub id: String,
    pub host: String,
    pub users_count: i32,
    pub notes_count: i32,
    pub following_count: i32,
    pub followers_count: i32,
    pub software_name: Option<String>,
    pub software_version: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub maintainer_email: Option<String>,
    pub maintainer_name: Option<String>,
    pub icon_url: Option<String>,
    pub favicon_url: Option<String>,
    pub theme_color: Option<String>,
    pub is_blocked: bool,
    pub is_silenced: bool,
    pub is_suspended: bool,
    pub moderation_note: Option<String>,
    pub last_communicated_at: Option<String>,
    pub info_updated_at: Option<String>,
    pub created_at: String,
}

impl From<instance::Model> for InstanceResponse {
    fn from(i: instance::Model) -> Self {
        Self {
            id: i.id,
            host: i.host,
            users_count: i.users_count,
            notes_count: i.notes_count,
            following_count: i.following_count,
            followers_count: i.followers_count,
            software_name: i.software_name,
            software_version: i.software_version,
            name: i.name,
            description: i.description,
            maintainer_email: i.maintainer_email,
            maintainer_name: i.maintainer_name,
            icon_url: i.icon_url,
            favicon_url: i.favicon_url,
            theme_color: i.theme_color,
            is_blocked: i.is_blocked,
            is_silenced: i.is_silenced,
            is_suspended: i.is_suspended,
            moderation_note: i.moderation_note,
            last_communicated_at: i.last_communicated_at.map(|t| t.to_rfc3339()),
            info_updated_at: i.info_updated_at.map(|t| t.to_rfc3339()),
            created_at: i.created_at.to_rfc3339(),
        }
    }
}

/// Federation stats response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FederationStatsResponse {
    pub total_instances: u64,
    pub blocked_instances: u64,
    pub silenced_instances: u64,
    pub suspended_instances: u64,
}

/// List instances request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListInstancesRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub sort_order: Option<String>,
    #[serde(default)]
    pub blocked: Option<bool>,
    #[serde(default)]
    pub silenced: Option<bool>,
    #[serde(default)]
    pub suspended: Option<bool>,
}

/// Show instance request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowInstanceRequest {
    pub host: String,
}

/// Update instance request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstanceRequest {
    pub host: String,
    #[serde(default)]
    pub is_blocked: Option<bool>,
    #[serde(default)]
    pub is_silenced: Option<bool>,
    #[serde(default)]
    pub is_suspended: Option<bool>,
    #[serde(default)]
    pub moderation_note: Option<String>,
}

/// Block/Unblock instance request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceHostRequest {
    pub host: String,
}

// ========== Report Endpoints ==========

/// Create an abuse report.
async fn create_report(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateReportRequest>,
) -> AppResult<ApiResponse<ReportResponse>> {
    let report = state
        .moderation_service
        .create_report(
            &user.id,
            CreateReportInput {
                target_user_id: req.user_id,
                target_note_id: req.note_id,
                comment: req.comment,
            },
        )
        .await?;

    Ok(ApiResponse::ok(report.into()))
}

/// Get pending reports (admin only).
async fn get_reports(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<GetReportsRequest>,
) -> AppResult<ApiResponse<Vec<ReportResponse>>> {
    // Verify admin/moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only moderators can view reports".to_string(),
        ));
    }

    let status = req.status.as_ref().and_then(|s| match s.as_str() {
        "pending" => Some(ReportStatus::Pending),
        "resolved" => Some(ReportStatus::Resolved),
        "rejected" => Some(ReportStatus::Rejected),
        _ => None,
    });

    let reports = state
        .moderation_service
        .get_reports(status, req.limit.min(100), req.offset)
        .await?;

    let responses: Vec<ReportResponse> = reports.into_iter().map(std::convert::Into::into).collect();

    Ok(ApiResponse::ok(responses))
}

/// Get a specific report (admin only).
async fn get_report(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<GetReportRequest>,
) -> AppResult<ApiResponse<ReportResponse>> {
    // Verify admin/moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only moderators can view reports".to_string(),
        ));
    }

    let report = state.moderation_service.get_report(&req.report_id).await?;

    Ok(ApiResponse::ok(report.into()))
}

/// Resolve a report (admin only).
async fn resolve_report(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ResolveReportRequest>,
) -> AppResult<ApiResponse<ReportResponse>> {
    let resolution = match req.resolution.as_str() {
        "resolved" => ReportStatus::Resolved,
        "rejected" => ReportStatus::Rejected,
        _ => {
            return Err(misskey_common::AppError::BadRequest(
                "Invalid resolution status".to_string(),
            ))
        }
    };

    let report = state
        .moderation_service
        .resolve_report(
            &user.id,
            ResolveReportInput {
                report_id: req.report_id,
                resolution,
                comment: req.comment,
            },
        )
        .await?;

    Ok(ApiResponse::ok(report.into()))
}

// ========== Suspension Endpoints ==========

/// Suspend a user (admin only).
async fn suspend_user(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<SuspendUserRequest>,
) -> AppResult<ApiResponse<SuspensionResponse>> {
    let suspension = state
        .moderation_service
        .suspend_user(
            &user.id,
            CreateSuspensionInput {
                user_id: req.user_id,
                reason: req.reason,
                duration: req.duration,
            },
        )
        .await?;

    Ok(ApiResponse::ok(suspension.into()))
}

/// Unsuspend a user (admin only).
async fn unsuspend_user(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UnsuspendUserRequest>,
) -> AppResult<ApiResponse<SuspensionResponse>> {
    let suspension = state
        .moderation_service
        .unsuspend_user(&user.id, &req.user_id)
        .await?;

    Ok(ApiResponse::ok(suspension.into()))
}

/// Get active suspensions (admin only).
async fn get_suspensions(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<GetReportsRequest>,
) -> AppResult<ApiResponse<Vec<SuspensionResponse>>> {
    // Verify admin/moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only moderators can view suspensions".to_string(),
        ));
    }

    let suspensions = state
        .moderation_service
        .get_active_suspensions(req.limit.min(100), req.offset)
        .await?;

    let responses: Vec<SuspensionResponse> = suspensions.into_iter().map(std::convert::Into::into).collect();

    Ok(ApiResponse::ok(responses))
}

// ========== Admin Stats ==========

/// Get admin queue stats (admin only).
async fn admin_stats(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<AdminStatsResponse>> {
    // Verify admin/moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only moderators can view admin stats".to_string(),
        ));
    }

    let pending_reports = state.moderation_service.count_pending_reports().await?;
    let active_suspensions = state
        .moderation_service
        .get_active_suspensions(1000, 0)
        .await?
        .len() as u64;

    Ok(ApiResponse::ok(AdminStatsResponse {
        pending_reports,
        active_suspensions,
    }))
}

// ========== Instance Endpoints ==========

/// List instances (admin only).
async fn list_instances(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListInstancesRequest>,
) -> AppResult<ApiResponse<Vec<InstanceResponse>>> {
    // Verify admin/moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only moderators can view instances".to_string(),
        ));
    }

    let limit = req.limit.min(100);
    let instances = if req.blocked == Some(true) {
        state.instance_service.list_blocked(limit, req.offset).await?
    } else if req.silenced == Some(true) {
        state.instance_service.list_silenced(limit, req.offset).await?
    } else if req.suspended == Some(true) {
        state.instance_service.list_suspended(limit, req.offset).await?
    } else {
        state
            .instance_service
            .list_all(limit, req.offset, req.sort.as_deref(), req.sort_order.as_deref())
            .await?
    };

    Ok(ApiResponse::ok(
        instances.into_iter().map(Into::into).collect(),
    ))
}

/// Show instance details (admin only).
async fn show_instance(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ShowInstanceRequest>,
) -> AppResult<ApiResponse<InstanceResponse>> {
    // Verify admin/moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only moderators can view instance details".to_string(),
        ));
    }

    let instance = state.instance_service.get_by_host(&req.host).await?;

    Ok(ApiResponse::ok(instance.into()))
}

/// Update instance moderation status (admin only).
async fn update_instance(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateInstanceRequest>,
) -> AppResult<ApiResponse<InstanceResponse>> {
    let instance = state
        .instance_service
        .update_instance(
            &user.id,
            UpdateInstanceInput {
                host: req.host,
                is_blocked: req.is_blocked,
                is_silenced: req.is_silenced,
                is_suspended: req.is_suspended,
                moderation_note: req.moderation_note,
            },
        )
        .await?;

    Ok(ApiResponse::ok(instance.into()))
}

/// Block an instance (admin only).
async fn block_instance(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<InstanceHostRequest>,
) -> AppResult<ApiResponse<InstanceResponse>> {
    let instance = state
        .instance_service
        .block_instance(&user.id, &req.host)
        .await?;

    Ok(ApiResponse::ok(instance.into()))
}

/// Unblock an instance (admin only).
async fn unblock_instance(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<InstanceHostRequest>,
) -> AppResult<ApiResponse<InstanceResponse>> {
    let instance = state
        .instance_service
        .unblock_instance(&user.id, &req.host)
        .await?;

    Ok(ApiResponse::ok(instance.into()))
}

/// Silence an instance (admin only).
async fn silence_instance(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<InstanceHostRequest>,
) -> AppResult<ApiResponse<InstanceResponse>> {
    let instance = state
        .instance_service
        .silence_instance(&user.id, &req.host)
        .await?;

    Ok(ApiResponse::ok(instance.into()))
}

/// Unsilence an instance (admin only).
async fn unsilence_instance(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<InstanceHostRequest>,
) -> AppResult<ApiResponse<InstanceResponse>> {
    let instance = state
        .instance_service
        .unsilence_instance(&user.id, &req.host)
        .await?;

    Ok(ApiResponse::ok(instance.into()))
}

/// Get federation statistics (admin only).
async fn federation_stats(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> AppResult<ApiResponse<FederationStatsResponse>> {
    // Verify admin/moderator
    if !user.is_admin && !user.is_moderator {
        return Err(misskey_common::AppError::Forbidden(
            "Only moderators can view federation stats".to_string(),
        ));
    }

    let stats = state.instance_service.get_stats().await?;

    Ok(ApiResponse::ok(FederationStatsResponse {
        total_instances: stats.total,
        blocked_instances: stats.blocked,
        silenced_instances: stats.silenced,
        suspended_instances: stats.suspended,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        // Reports
        .route("/abuse-reports/create", post(create_report))
        .route("/abuse-reports/list", post(get_reports))
        .route("/abuse-reports/show", post(get_report))
        .route("/abuse-reports/resolve", post(resolve_report))
        // Suspensions
        .route("/suspend-user", post(suspend_user))
        .route("/unsuspend-user", post(unsuspend_user))
        .route("/suspensions/list", post(get_suspensions))
        // Instance/Federation management
        .route("/federation/instances", post(list_instances))
        .route("/federation/show-instance", post(show_instance))
        .route("/federation/update-instance", post(update_instance))
        .route("/federation/block-instance", post(block_instance))
        .route("/federation/unblock-instance", post(unblock_instance))
        .route("/federation/silence-instance", post(silence_instance))
        .route("/federation/unsilence-instance", post(unsilence_instance))
        .route("/federation/stats", post(federation_stats))
        // Stats
        .route("/queue/stats", post(admin_stats))
}
