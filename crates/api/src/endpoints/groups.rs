//! Group endpoints.

use axum::{Json, Router, extract::State, routing::post};
use misskey_common::AppResult;
use misskey_core::services::group::{
    CreateGroupInput, GroupResponse, InviteUserInput, JoinRequestInput, UpdateGroupInput,
    UpdateMemberRoleInput,
};
use misskey_db::entities::group::GroupJoinPolicy;
use misskey_db::entities::group_member::GroupRole;
use misskey_db::entities::{group, group_invite, group_member};
use serde::{Deserialize, Serialize};

use crate::{extractors::AuthUser, middleware::AppState, response::ApiResponse};

// ==================== Request/Response Types ====================

/// Group list response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupListResponse {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    pub description: Option<String>,
    pub banner_id: Option<String>,
    pub avatar_id: Option<String>,
    pub join_policy: GroupJoinPolicy,
    pub is_searchable: bool,
    pub members_only_post: bool,
    pub members_count: i64,
    pub notes_count: i64,
    pub created_at: String,
}

impl From<group::Model> for GroupListResponse {
    fn from(g: group::Model) -> Self {
        Self {
            id: g.id,
            owner_id: g.owner_id,
            name: g.name,
            description: g.description,
            banner_id: g.banner_id,
            avatar_id: g.avatar_id,
            join_policy: g.join_policy,
            is_searchable: g.is_searchable,
            members_only_post: g.members_only_post,
            members_count: g.members_count,
            notes_count: g.notes_count,
            created_at: g.created_at.to_rfc3339(),
        }
    }
}

/// Full group response with membership info.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupDetailResponse {
    pub id: String,
    pub owner_id: String,
    pub name: String,
    pub description: Option<String>,
    pub banner_id: Option<String>,
    pub avatar_id: Option<String>,
    pub join_policy: GroupJoinPolicy,
    pub is_searchable: bool,
    pub members_only_post: bool,
    pub members_count: i64,
    pub notes_count: i64,
    pub rules: Option<String>,
    pub created_at: String,
    pub is_member: bool,
    pub my_role: Option<GroupRole>,
}

impl From<GroupResponse> for GroupDetailResponse {
    fn from(g: GroupResponse) -> Self {
        Self {
            id: g.id,
            owner_id: g.owner_id,
            name: g.name,
            description: g.description,
            banner_id: g.banner_id,
            avatar_id: g.avatar_id,
            join_policy: g.join_policy,
            is_searchable: g.is_searchable,
            members_only_post: g.members_only_post,
            members_count: g.members_count,
            notes_count: g.notes_count,
            rules: g.rules,
            created_at: g.created_at.to_rfc3339(),
            is_member: g.is_member,
            my_role: g.my_role,
        }
    }
}

/// Member response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberResponse {
    pub id: String,
    pub user_id: String,
    pub group_id: String,
    pub role: GroupRole,
    pub is_muted: bool,
    pub nickname: Option<String>,
    pub joined_at: String,
}

impl From<group_member::Model> for MemberResponse {
    fn from(m: group_member::Model) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            group_id: m.group_id,
            role: m.role,
            is_muted: m.is_muted,
            nickname: m.nickname,
            joined_at: m.joined_at.to_rfc3339(),
        }
    }
}

/// Invite response.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteResponse {
    pub id: String,
    pub group_id: String,
    pub user_id: String,
    pub inviter_id: Option<String>,
    pub invite_type: String,
    pub status: String,
    pub message: Option<String>,
    pub created_at: String,
}

impl From<group_invite::Model> for InviteResponse {
    fn from(i: group_invite::Model) -> Self {
        Self {
            id: i.id,
            group_id: i.group_id,
            user_id: i.user_id,
            inviter_id: i.inviter_id,
            invite_type: format!("{:?}", i.invite_type).to_lowercase(),
            status: format!("{:?}", i.status).to_lowercase(),
            message: i.message,
            created_at: i.created_at.to_rfc3339(),
        }
    }
}

/// Show group request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowGroupRequest {
    pub group_id: String,
}

/// Delete group request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteGroupRequest {
    pub group_id: String,
}

/// List groups request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListGroupsRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Search groups request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchGroupsRequest {
    #[serde(default)]
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Join group request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinGroupRequest {
    pub group_id: String,
}

/// Leave group request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaveGroupRequest {
    pub group_id: String,
}

/// Kick member request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KickMemberRequest {
    pub group_id: String,
    pub user_id: String,
}

/// List members request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMembersRequest {
    pub group_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Accept/Reject invite request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteActionRequest {
    pub invite_id: String,
}

/// List invites request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListInvitesRequest {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// List join requests request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListJoinRequestsRequest {
    pub group_id: String,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

/// Transfer ownership request.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferOwnershipRequest {
    pub group_id: String,
    pub user_id: String,
}

const fn default_limit() -> u64 {
    10
}

// ==================== Handlers ====================

/// Create a new group.
async fn create(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateGroupInput>,
) -> AppResult<ApiResponse<GroupListResponse>> {
    let group = state.group_service.create(&user.id, input).await?;

    Ok(ApiResponse::ok(group.into()))
}

/// Update a group.
async fn update(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<UpdateGroupInput>,
) -> AppResult<ApiResponse<GroupListResponse>> {
    let group = state.group_service.update(&user.id, input).await?;

    Ok(ApiResponse::ok(group.into()))
}

/// Delete a group.
async fn delete(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<DeleteGroupRequest>,
) -> AppResult<ApiResponse<()>> {
    state.group_service.delete(&req.group_id, &user.id).await?;

    Ok(ApiResponse::ok(()))
}

/// Show a group.
async fn show(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ShowGroupRequest>,
) -> AppResult<ApiResponse<GroupDetailResponse>> {
    let response = state
        .group_service
        .get_with_member_info(&req.group_id, &user.id)
        .await?;

    Ok(ApiResponse::ok(response.into()))
}

/// List owned groups.
async fn owned(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListGroupsRequest>,
) -> AppResult<ApiResponse<Vec<GroupListResponse>>> {
    let limit = req.limit.min(100);
    let groups = state
        .group_service
        .list_owned(&user.id, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(
        groups.into_iter().map(Into::into).collect(),
    ))
}

/// List joined groups.
async fn joined(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListGroupsRequest>,
) -> AppResult<ApiResponse<Vec<GroupListResponse>>> {
    let limit = req.limit.min(100);
    let groups = state
        .group_service
        .list_joined(&user.id, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(
        groups.into_iter().map(Into::into).collect(),
    ))
}

/// List featured groups.
async fn featured(
    State(state): State<AppState>,
    Json(req): Json<ListGroupsRequest>,
) -> AppResult<ApiResponse<Vec<GroupListResponse>>> {
    let limit = req.limit.min(100);
    let groups = state.group_service.list_featured(limit, req.offset).await?;

    Ok(ApiResponse::ok(
        groups.into_iter().map(Into::into).collect(),
    ))
}

/// Search groups.
async fn search(
    State(state): State<AppState>,
    Json(req): Json<SearchGroupsRequest>,
) -> AppResult<ApiResponse<Vec<GroupListResponse>>> {
    let limit = req.limit.min(100);
    let groups = state
        .group_service
        .search(&req.query, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(
        groups.into_iter().map(Into::into).collect(),
    ))
}

/// Join an open group.
async fn join(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<JoinGroupRequest>,
) -> AppResult<ApiResponse<MemberResponse>> {
    let member = state.group_service.join(&user.id, &req.group_id).await?;

    Ok(ApiResponse::ok(member.into()))
}

/// Leave a group.
async fn leave(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<LeaveGroupRequest>,
) -> AppResult<ApiResponse<()>> {
    state.group_service.leave(&user.id, &req.group_id).await?;

    Ok(ApiResponse::ok(()))
}

/// Invite a user to a group.
async fn invite(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<InviteUserInput>,
) -> AppResult<ApiResponse<InviteResponse>> {
    let invite = state.group_service.invite(&user.id, input).await?;

    Ok(ApiResponse::ok(invite.into()))
}

/// Request to join a group.
async fn request_join(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<JoinRequestInput>,
) -> AppResult<ApiResponse<InviteResponse>> {
    let invite = state.group_service.request_join(&user.id, input).await?;

    Ok(ApiResponse::ok(invite.into()))
}

/// Accept an invitation.
async fn accept_invite(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<InviteActionRequest>,
) -> AppResult<ApiResponse<MemberResponse>> {
    let member = state
        .group_service
        .accept_invite(&user.id, &req.invite_id)
        .await?;

    Ok(ApiResponse::ok(member.into()))
}

/// Reject an invitation.
async fn reject_invite(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<InviteActionRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .group_service
        .reject_invite(&user.id, &req.invite_id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Approve a join request (admin).
async fn approve_request(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<InviteActionRequest>,
) -> AppResult<ApiResponse<MemberResponse>> {
    let member = state
        .group_service
        .approve_request(&user.id, &req.invite_id)
        .await?;

    Ok(ApiResponse::ok(member.into()))
}

/// Reject a join request (admin).
async fn reject_request(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<InviteActionRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .group_service
        .reject_request(&user.id, &req.invite_id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Kick a member.
async fn kick(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<KickMemberRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .group_service
        .kick(&user.id, &req.group_id, &req.user_id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// Update member role.
async fn update_role(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(input): Json<UpdateMemberRoleInput>,
) -> AppResult<ApiResponse<MemberResponse>> {
    let member = state.group_service.update_role(&user.id, input).await?;

    Ok(ApiResponse::ok(member.into()))
}

/// Transfer ownership.
async fn transfer(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<TransferOwnershipRequest>,
) -> AppResult<ApiResponse<()>> {
    state
        .group_service
        .transfer_ownership(&user.id, &req.group_id, &req.user_id)
        .await?;

    Ok(ApiResponse::ok(()))
}

/// List members of a group.
async fn members(
    State(state): State<AppState>,
    Json(req): Json<ListMembersRequest>,
) -> AppResult<ApiResponse<Vec<MemberResponse>>> {
    let limit = req.limit.min(100);
    let members = state
        .group_service
        .list_members(&req.group_id, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(
        members.into_iter().map(Into::into).collect(),
    ))
}

/// List pending invitations for current user.
async fn invitations(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListInvitesRequest>,
) -> AppResult<ApiResponse<Vec<InviteResponse>>> {
    let limit = req.limit.min(100);
    let invites = state
        .group_service
        .list_my_invitations(&user.id, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(
        invites.into_iter().map(Into::into).collect(),
    ))
}

/// List pending join requests for a group (admin).
async fn join_requests(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ListJoinRequestsRequest>,
) -> AppResult<ApiResponse<Vec<InviteResponse>>> {
    let limit = req.limit.min(100);
    let invites = state
        .group_service
        .list_join_requests(&user.id, &req.group_id, limit, req.offset)
        .await?;

    Ok(ApiResponse::ok(
        invites.into_iter().map(Into::into).collect(),
    ))
}

pub fn router() -> Router<AppState> {
    Router::new()
        // Group CRUD
        .route("/create", post(create))
        .route("/update", post(update))
        .route("/delete", post(delete))
        .route("/show", post(show))
        // Listing
        .route("/owned", post(owned))
        .route("/joined", post(joined))
        .route("/featured", post(featured))
        .route("/search", post(search))
        // Membership
        .route("/join", post(join))
        .route("/leave", post(leave))
        .route("/invite", post(invite))
        .route("/request", post(request_join))
        // Invite actions
        .route("/invitations/accept", post(accept_invite))
        .route("/invitations/reject", post(reject_invite))
        .route("/requests/approve", post(approve_request))
        .route("/requests/reject", post(reject_request))
        // Management
        .route("/kick", post(kick))
        .route("/members", post(members))
        .route("/members/update-role", post(update_role))
        .route("/transfer", post(transfer))
        // Listings
        .route("/invitations", post(invitations))
        .route("/requests", post(join_requests))
}
