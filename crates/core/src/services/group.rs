//! Group service.

use chrono::Utc;
use misskey_common::{AppError, AppResult, id::IdGenerator};
use misskey_db::entities::group::GroupJoinPolicy;
use misskey_db::entities::group_invite::{InviteStatus, InviteType};
use misskey_db::entities::group_member::GroupRole;
use misskey_db::entities::{group, group_invite, group_member};
use misskey_db::repositories::GroupRepository;
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Maximum number of groups a user can own.
const MAX_GROUPS_PER_USER: u64 = 5;

/// Maximum number of groups a user can join.
const MAX_JOINED_GROUPS: u64 = 50;

/// Input for creating a group.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupInput {
    #[validate(length(min = 1, max = 128))]
    pub name: String,
    #[validate(length(max = 2048))]
    pub description: Option<String>,
    pub banner_id: Option<String>,
    pub avatar_id: Option<String>,
    #[serde(default)]
    pub join_policy: GroupJoinPolicy,
    #[serde(default = "default_true")]
    pub is_searchable: bool,
    #[serde(default = "default_true")]
    pub members_only_post: bool,
    #[validate(length(max = 4096))]
    pub rules: Option<String>,
}

const fn default_true() -> bool {
    true
}

/// Input for updating a group.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGroupInput {
    pub group_id: String,
    #[validate(length(min = 1, max = 128))]
    pub name: Option<String>,
    #[validate(length(max = 2048))]
    pub description: Option<Option<String>>,
    pub banner_id: Option<Option<String>>,
    pub avatar_id: Option<Option<String>>,
    pub join_policy: Option<GroupJoinPolicy>,
    pub is_searchable: Option<bool>,
    pub members_only_post: Option<bool>,
    #[validate(length(max = 4096))]
    pub rules: Option<Option<String>>,
}

/// Input for inviting a user to a group.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteUserInput {
    pub group_id: String,
    pub user_id: String,
    pub message: Option<String>,
}

/// Input for requesting to join a group.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRequestInput {
    pub group_id: String,
    pub message: Option<String>,
}

/// Input for updating member role.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemberRoleInput {
    pub group_id: String,
    pub user_id: String,
    pub role: GroupRole,
}

/// Group response with member info.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupResponse {
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
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_member: bool,
    pub my_role: Option<GroupRole>,
}

impl GroupResponse {
    #[must_use]
    pub fn from_model(model: group::Model, is_member: bool, my_role: Option<GroupRole>) -> Self {
        Self {
            id: model.id,
            owner_id: model.owner_id,
            name: model.name,
            description: model.description,
            banner_id: model.banner_id,
            avatar_id: model.avatar_id,
            join_policy: model.join_policy,
            is_searchable: model.is_searchable,
            members_only_post: model.members_only_post,
            members_count: model.members_count,
            notes_count: model.notes_count,
            rules: model.rules,
            created_at: model.created_at.into(),
            is_member,
            my_role,
        }
    }
}

/// Service for managing groups.
#[derive(Clone)]
pub struct GroupService {
    group_repo: GroupRepository,
    id_gen: IdGenerator,
}

impl GroupService {
    /// Create a new group service.
    #[must_use]
    pub const fn new(group_repo: GroupRepository) -> Self {
        Self {
            group_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Get a group by ID.
    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<group::Model>> {
        self.group_repo.find_by_id(id).await
    }

    /// Get a group by ID with member info.
    pub async fn get_with_member_info(&self, id: &str, user_id: &str) -> AppResult<GroupResponse> {
        let group = self.group_repo.get_by_id(id).await?;
        let member = self.group_repo.get_member(user_id, id).await?;
        let is_member = member.is_some();
        let my_role = member.map(|m| m.role);

        Ok(GroupResponse::from_model(group, is_member, my_role))
    }

    /// List groups owned by user.
    pub async fn list_owned(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<group::Model>> {
        self.group_repo
            .find_owned_by_user(user_id, limit, offset)
            .await
    }

    /// List groups user is a member of.
    pub async fn list_joined(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<group::Model>> {
        self.group_repo
            .find_joined_by_user(user_id, limit, offset)
            .await
    }

    /// List featured groups.
    pub async fn list_featured(&self, limit: u64, offset: u64) -> AppResult<Vec<group::Model>> {
        self.group_repo.find_featured(limit, offset).await
    }

    /// Search groups.
    pub async fn search(
        &self,
        query: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<group::Model>> {
        if query.trim().is_empty() {
            return self.list_featured(limit, offset).await;
        }

        self.group_repo.search(query, limit, offset).await
    }

    /// Create a new group.
    pub async fn create(&self, user_id: &str, input: CreateGroupInput) -> AppResult<group::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Check group limit
        let count = self.group_repo.count_owned_by_user(user_id).await?;
        if count >= MAX_GROUPS_PER_USER {
            return Err(AppError::Validation(format!(
                "Maximum of {MAX_GROUPS_PER_USER} groups allowed per user"
            )));
        }

        let group_id = self.id_gen.generate();
        let now = Utc::now();

        // Create group
        let model = group::ActiveModel {
            id: Set(group_id.clone()),
            owner_id: Set(user_id.to_string()),
            name: Set(input.name),
            description: Set(input.description),
            banner_id: Set(input.banner_id),
            avatar_id: Set(input.avatar_id),
            join_policy: Set(input.join_policy),
            is_archived: Set(false),
            is_searchable: Set(input.is_searchable),
            members_only_post: Set(input.members_only_post),
            members_count: Set(1), // Owner is the first member
            notes_count: Set(0),
            rules: Set(input.rules),
            metadata: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(None),
        };

        let group = self.group_repo.create(model).await?;

        // Add owner as a member with owner role
        let member_model = group_member::ActiveModel {
            id: Set(self.id_gen.generate()),
            user_id: Set(user_id.to_string()),
            group_id: Set(group_id),
            role: Set(GroupRole::Owner),
            is_muted: Set(false),
            is_banned: Set(false),
            nickname: Set(None),
            joined_at: Set(now.into()),
            updated_at: Set(None),
        };

        // Don't increment member count since it's already 1
        member_model
            .insert(self.group_repo.db())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(group)
    }

    /// Update a group.
    pub async fn update(&self, user_id: &str, input: UpdateGroupInput) -> AppResult<group::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Check permission
        self.require_manage_settings(&input.group_id, user_id)
            .await?;

        let group = self.group_repo.get_by_id(&input.group_id).await?;
        let now = Utc::now();
        let mut active: group::ActiveModel = group.into();

        if let Some(name) = input.name {
            active.name = Set(name);
        }
        if let Some(description) = input.description {
            active.description = Set(description);
        }
        if let Some(banner_id) = input.banner_id {
            active.banner_id = Set(banner_id);
        }
        if let Some(avatar_id) = input.avatar_id {
            active.avatar_id = Set(avatar_id);
        }
        if let Some(join_policy) = input.join_policy {
            active.join_policy = Set(join_policy);
        }
        if let Some(is_searchable) = input.is_searchable {
            active.is_searchable = Set(is_searchable);
        }
        if let Some(members_only_post) = input.members_only_post {
            active.members_only_post = Set(members_only_post);
        }
        if let Some(rules) = input.rules {
            active.rules = Set(rules);
        }

        active.updated_at = Set(Some(now.into()));

        self.group_repo.update(active).await
    }

    /// Delete a group (archive).
    pub async fn delete(&self, group_id: &str, user_id: &str) -> AppResult<()> {
        // Only owner can delete
        let group = self.group_repo.get_by_id(group_id).await?;
        if group.owner_id != user_id {
            return Err(AppError::Forbidden(
                "Only the owner can delete the group".to_string(),
            ));
        }

        self.group_repo.archive(group_id).await?;
        Ok(())
    }

    // ==================== Member Operations ====================

    /// Invite a user to a group.
    pub async fn invite(
        &self,
        inviter_id: &str,
        input: InviteUserInput,
    ) -> AppResult<group_invite::Model> {
        // Check inviter has permission
        self.require_manage_members(&input.group_id, inviter_id)
            .await?;

        // Check if user is already a member
        if self
            .group_repo
            .is_member(&input.user_id, &input.group_id)
            .await?
        {
            return Err(AppError::Validation("User is already a member".to_string()));
        }

        // Check if there's already a pending invite
        if let Some(_) = self
            .group_repo
            .get_pending_invite(&input.user_id, &input.group_id)
            .await?
        {
            return Err(AppError::Validation(
                "User already has a pending invite".to_string(),
            ));
        }

        let now = Utc::now();
        let model = group_invite::ActiveModel {
            id: Set(self.id_gen.generate()),
            group_id: Set(input.group_id),
            user_id: Set(input.user_id),
            inviter_id: Set(Some(inviter_id.to_string())),
            invite_type: Set(InviteType::Invite),
            status: Set(InviteStatus::Pending),
            message: Set(input.message),
            expires_at: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(None),
        };

        self.group_repo.create_invite(model).await
    }

    /// Request to join a group.
    pub async fn request_join(
        &self,
        user_id: &str,
        input: JoinRequestInput,
    ) -> AppResult<group_invite::Model> {
        let group = self.group_repo.get_by_id(&input.group_id).await?;

        // Check if group accepts join requests
        match group.join_policy {
            GroupJoinPolicy::InviteOnly => {
                return Err(AppError::Forbidden("This group is invite only".to_string()));
            }
            GroupJoinPolicy::Open => {
                // Auto-join for open groups
                return Err(AppError::Validation(
                    "Use join endpoint for open groups".to_string(),
                ));
            }
            GroupJoinPolicy::Approval => {
                // Continue with request
            }
        }

        // Check if already a member
        if self.group_repo.is_member(user_id, &input.group_id).await? {
            return Err(AppError::Validation("Already a member".to_string()));
        }

        // Check if there's already a pending request
        if let Some(_) = self
            .group_repo
            .get_pending_invite(user_id, &input.group_id)
            .await?
        {
            return Err(AppError::Validation(
                "Already have a pending request".to_string(),
            ));
        }

        let now = Utc::now();
        let model = group_invite::ActiveModel {
            id: Set(self.id_gen.generate()),
            group_id: Set(input.group_id),
            user_id: Set(user_id.to_string()),
            inviter_id: Set(None),
            invite_type: Set(InviteType::Request),
            status: Set(InviteStatus::Pending),
            message: Set(input.message),
            expires_at: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(None),
        };

        self.group_repo.create_invite(model).await
    }

    /// Join an open group directly.
    pub async fn join(&self, user_id: &str, group_id: &str) -> AppResult<group_member::Model> {
        let group = self.group_repo.get_by_id(group_id).await?;

        // Check if group is open
        if group.join_policy != GroupJoinPolicy::Open {
            return Err(AppError::Forbidden(
                "Group is not open for direct joining".to_string(),
            ));
        }

        // Check if already a member
        if self.group_repo.is_member(user_id, group_id).await? {
            return Err(AppError::Validation("Already a member".to_string()));
        }

        // Create member
        let now = Utc::now();
        let model = group_member::ActiveModel {
            id: Set(self.id_gen.generate()),
            user_id: Set(user_id.to_string()),
            group_id: Set(group_id.to_string()),
            role: Set(GroupRole::Member),
            is_muted: Set(false),
            is_banned: Set(false),
            nickname: Set(None),
            joined_at: Set(now.into()),
            updated_at: Set(None),
        };

        self.group_repo.add_member(model).await
    }

    /// Accept an invitation.
    pub async fn accept_invite(
        &self,
        user_id: &str,
        invite_id: &str,
    ) -> AppResult<group_member::Model> {
        let invite = self
            .group_repo
            .get_invite_by_id(invite_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invite not found".to_string()))?;

        // Check if this is for the user
        if invite.user_id != user_id {
            return Err(AppError::Forbidden("Not your invitation".to_string()));
        }

        // Check status
        if invite.status != InviteStatus::Pending {
            return Err(AppError::Validation(
                "Invitation is no longer pending".to_string(),
            ));
        }

        // Must be an invite (not a request)
        if invite.invite_type != InviteType::Invite {
            return Err(AppError::Validation(
                "This is a join request, not an invitation".to_string(),
            ));
        }

        // Update invite status
        self.group_repo
            .update_invite_status(invite_id, InviteStatus::Accepted)
            .await?;

        // Add as member
        let now = Utc::now();
        let model = group_member::ActiveModel {
            id: Set(self.id_gen.generate()),
            user_id: Set(user_id.to_string()),
            group_id: Set(invite.group_id),
            role: Set(GroupRole::Member),
            is_muted: Set(false),
            is_banned: Set(false),
            nickname: Set(None),
            joined_at: Set(now.into()),
            updated_at: Set(None),
        };

        self.group_repo.add_member(model).await
    }

    /// Reject an invitation.
    pub async fn reject_invite(&self, user_id: &str, invite_id: &str) -> AppResult<()> {
        let invite = self
            .group_repo
            .get_invite_by_id(invite_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invite not found".to_string()))?;

        // Check if this is for the user
        if invite.user_id != user_id {
            return Err(AppError::Forbidden("Not your invitation".to_string()));
        }

        // Check status
        if invite.status != InviteStatus::Pending {
            return Err(AppError::Validation(
                "Invitation is no longer pending".to_string(),
            ));
        }

        self.group_repo
            .update_invite_status(invite_id, InviteStatus::Rejected)
            .await?;
        Ok(())
    }

    /// Approve a join request (by group admin).
    pub async fn approve_request(
        &self,
        approver_id: &str,
        invite_id: &str,
    ) -> AppResult<group_member::Model> {
        let invite = self
            .group_repo
            .get_invite_by_id(invite_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Request not found".to_string()))?;

        // Check approver has permission
        self.require_manage_members(&invite.group_id, approver_id)
            .await?;

        // Check status
        if invite.status != InviteStatus::Pending {
            return Err(AppError::Validation(
                "Request is no longer pending".to_string(),
            ));
        }

        // Must be a request (not an invite)
        if invite.invite_type != InviteType::Request {
            return Err(AppError::Validation(
                "This is an invitation, not a join request".to_string(),
            ));
        }

        // Update invite status
        self.group_repo
            .update_invite_status(invite_id, InviteStatus::Accepted)
            .await?;

        // Add as member
        let now = Utc::now();
        let model = group_member::ActiveModel {
            id: Set(self.id_gen.generate()),
            user_id: Set(invite.user_id),
            group_id: Set(invite.group_id),
            role: Set(GroupRole::Member),
            is_muted: Set(false),
            is_banned: Set(false),
            nickname: Set(None),
            joined_at: Set(now.into()),
            updated_at: Set(None),
        };

        self.group_repo.add_member(model).await
    }

    /// Reject a join request (by group admin).
    pub async fn reject_request(&self, rejecter_id: &str, invite_id: &str) -> AppResult<()> {
        let invite = self
            .group_repo
            .get_invite_by_id(invite_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Request not found".to_string()))?;

        // Check rejecter has permission
        self.require_manage_members(&invite.group_id, rejecter_id)
            .await?;

        // Check status
        if invite.status != InviteStatus::Pending {
            return Err(AppError::Validation(
                "Request is no longer pending".to_string(),
            ));
        }

        self.group_repo
            .update_invite_status(invite_id, InviteStatus::Rejected)
            .await?;
        Ok(())
    }

    /// Leave a group.
    pub async fn leave(&self, user_id: &str, group_id: &str) -> AppResult<()> {
        let member = self
            .group_repo
            .get_member(user_id, group_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Not a member".to_string()))?;

        // Owner cannot leave (must transfer first)
        if member.role == GroupRole::Owner {
            return Err(AppError::Validation(
                "Owner cannot leave. Transfer ownership first.".to_string(),
            ));
        }

        self.group_repo.remove_member(user_id, group_id).await
    }

    /// Kick a member from the group.
    pub async fn kick(&self, kicker_id: &str, group_id: &str, user_id: &str) -> AppResult<()> {
        // Check kicker has permission
        self.require_manage_members(group_id, kicker_id).await?;

        let member = self
            .group_repo
            .get_member(user_id, group_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User is not a member".to_string()))?;

        // Cannot kick owner
        if member.role == GroupRole::Owner {
            return Err(AppError::Forbidden("Cannot kick the owner".to_string()));
        }

        // Check kicker has higher role
        let kicker_role = self
            .group_repo
            .get_member_role(kicker_id, group_id)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not authorized".to_string()))?;

        if !can_manage(&kicker_role, &member.role) {
            return Err(AppError::Forbidden(
                "Cannot kick members with equal or higher role".to_string(),
            ));
        }

        self.group_repo.remove_member(user_id, group_id).await
    }

    /// Update a member's role.
    pub async fn update_role(
        &self,
        updater_id: &str,
        input: UpdateMemberRoleInput,
    ) -> AppResult<group_member::Model> {
        // Check updater has permission
        self.require_manage_members(&input.group_id, updater_id)
            .await?;

        let member = self
            .group_repo
            .get_member(&input.user_id, &input.group_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User is not a member".to_string()))?;

        let updater_role = self
            .group_repo
            .get_member_role(updater_id, &input.group_id)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not authorized".to_string()))?;

        // Cannot change owner role directly
        if member.role == GroupRole::Owner {
            return Err(AppError::Forbidden(
                "Use transfer_ownership to change owner".to_string(),
            ));
        }

        // Cannot promote to owner
        if input.role == GroupRole::Owner {
            return Err(AppError::Forbidden(
                "Use transfer_ownership to promote to owner".to_string(),
            ));
        }

        // Check updater has higher role
        if !can_manage(&updater_role, &member.role) {
            return Err(AppError::Forbidden(
                "Cannot manage members with equal or higher role".to_string(),
            ));
        }

        self.group_repo
            .update_member_role(&input.user_id, &input.group_id, input.role)
            .await
    }

    /// Transfer group ownership.
    pub async fn transfer_ownership(
        &self,
        owner_id: &str,
        group_id: &str,
        new_owner_id: &str,
    ) -> AppResult<()> {
        let group = self.group_repo.get_by_id(group_id).await?;

        // Check current owner
        if group.owner_id != owner_id {
            return Err(AppError::Forbidden(
                "Only the owner can transfer ownership".to_string(),
            ));
        }

        // Check new owner is a member
        if !self.group_repo.is_member(new_owner_id, group_id).await? {
            return Err(AppError::Validation(
                "New owner must be a member".to_string(),
            ));
        }

        // Update group owner
        let mut active: group::ActiveModel = group.into();
        active.owner_id = Set(new_owner_id.to_string());
        active.updated_at = Set(Some(Utc::now().into()));
        self.group_repo.update(active).await?;

        // Update old owner role to admin
        self.group_repo
            .update_member_role(owner_id, group_id, GroupRole::Admin)
            .await?;

        // Update new owner role
        self.group_repo
            .update_member_role(new_owner_id, group_id, GroupRole::Owner)
            .await?;

        Ok(())
    }

    /// List members of a group.
    pub async fn list_members(
        &self,
        group_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<group_member::Model>> {
        self.group_repo.list_members(group_id, limit, offset).await
    }

    /// List pending invitations for user.
    pub async fn list_my_invitations(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<group_invite::Model>> {
        self.group_repo
            .list_pending_invites_for_user(user_id, limit, offset)
            .await
    }

    /// List pending join requests for a group.
    pub async fn list_join_requests(
        &self,
        user_id: &str,
        group_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<group_invite::Model>> {
        // Check permission
        self.require_manage_members(group_id, user_id).await?;

        self.group_repo
            .list_pending_requests_for_group(group_id, limit, offset)
            .await
    }

    // ==================== Permission Helpers ====================

    /// Check if user can manage members.
    async fn require_manage_members(&self, group_id: &str, user_id: &str) -> AppResult<()> {
        let role = self
            .group_repo
            .get_member_role(user_id, group_id)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this group".to_string()))?;

        if !role.can_manage_members() {
            return Err(AppError::Forbidden(
                "No permission to manage members".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if user can manage settings.
    async fn require_manage_settings(&self, group_id: &str, user_id: &str) -> AppResult<()> {
        let role = self
            .group_repo
            .get_member_role(user_id, group_id)
            .await?
            .ok_or_else(|| AppError::Forbidden("Not a member of this group".to_string()))?;

        if !role.can_manage_settings() {
            return Err(AppError::Forbidden(
                "No permission to manage settings".to_string(),
            ));
        }

        Ok(())
    }
}

/// Check if `actor_role` can manage `target_role`.
const fn can_manage(actor_role: &GroupRole, target_role: &GroupRole) -> bool {
    match actor_role {
        GroupRole::Owner => true, // Owner can manage everyone
        GroupRole::Admin => matches!(target_role, GroupRole::Member | GroupRole::Moderator),
        GroupRole::Moderator => matches!(target_role, GroupRole::Member),
        GroupRole::Member => false,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_can_manage() {
        // Owner can manage everyone
        assert!(can_manage(&GroupRole::Owner, &GroupRole::Admin));
        assert!(can_manage(&GroupRole::Owner, &GroupRole::Moderator));
        assert!(can_manage(&GroupRole::Owner, &GroupRole::Member));

        // Admin can manage moderators and members
        assert!(can_manage(&GroupRole::Admin, &GroupRole::Moderator));
        assert!(can_manage(&GroupRole::Admin, &GroupRole::Member));
        assert!(!can_manage(&GroupRole::Admin, &GroupRole::Admin));
        assert!(!can_manage(&GroupRole::Admin, &GroupRole::Owner));

        // Moderator can only manage members
        assert!(can_manage(&GroupRole::Moderator, &GroupRole::Member));
        assert!(!can_manage(&GroupRole::Moderator, &GroupRole::Moderator));

        // Member cannot manage anyone
        assert!(!can_manage(&GroupRole::Member, &GroupRole::Member));
    }

    #[test]
    fn test_role_capabilities() {
        assert!(GroupRole::Owner.can_moderate());
        assert!(GroupRole::Owner.can_manage_members());
        assert!(GroupRole::Owner.can_manage_settings());
        assert!(GroupRole::Owner.is_owner());

        assert!(GroupRole::Admin.can_moderate());
        assert!(GroupRole::Admin.can_manage_members());
        assert!(GroupRole::Admin.can_manage_settings());
        assert!(!GroupRole::Admin.is_owner());

        assert!(GroupRole::Moderator.can_moderate());
        assert!(!GroupRole::Moderator.can_manage_members());
        assert!(!GroupRole::Moderator.can_manage_settings());

        assert!(!GroupRole::Member.can_moderate());
        assert!(!GroupRole::Member.can_manage_members());
        assert!(!GroupRole::Member.can_manage_settings());
    }
}
