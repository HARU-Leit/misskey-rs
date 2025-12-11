//! Group repository.

use std::sync::Arc;

use chrono::Utc;
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::entities::group_invite::{InviteStatus, InviteType};
use crate::entities::group_member::GroupRole;
use crate::entities::{Group, GroupInvite, GroupMember, group, group_invite, group_member};

/// Repository for group operations.
#[derive(Clone)]
pub struct GroupRepository {
    db: Arc<DatabaseConnection>,
}

impl GroupRepository {
    /// Create a new group repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Get reference to the database connection.
    pub fn db(&self) -> &DatabaseConnection {
        self.db.as_ref()
    }

    // ==================== Group Operations ====================

    /// Find group by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<group::Model>> {
        Group::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get group by ID, returning error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<group::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Group not found: {id}")))
    }

    /// Find groups owned by user.
    pub async fn find_owned_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<group::Model>> {
        Group::find()
            .filter(group::Column::OwnerId.eq(user_id))
            .filter(group::Column::IsArchived.eq(false))
            .order_by(group::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find groups a user is a member of.
    pub async fn find_joined_by_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<group::Model>> {
        // Get group IDs the user is a member of
        let memberships = GroupMember::find()
            .filter(group_member::Column::UserId.eq(user_id))
            .filter(group_member::Column::IsBanned.eq(false))
            .order_by(group_member::Column::JoinedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let group_ids: Vec<String> = memberships.iter().map(|m| m.group_id.clone()).collect();

        if group_ids.is_empty() {
            return Ok(vec![]);
        }

        Group::find()
            .filter(group::Column::Id.is_in(group_ids))
            .filter(group::Column::IsArchived.eq(false))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find featured (popular) groups.
    pub async fn find_featured(&self, limit: u64, offset: u64) -> AppResult<Vec<group::Model>> {
        Group::find()
            .filter(group::Column::IsArchived.eq(false))
            .filter(group::Column::IsSearchable.eq(true))
            .order_by(group::Column::MembersCount, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Search groups by name.
    pub async fn search(
        &self,
        query: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<group::Model>> {
        Group::find()
            .filter(group::Column::Name.contains(query))
            .filter(group::Column::IsArchived.eq(false))
            .filter(group::Column::IsSearchable.eq(true))
            .order_by(group::Column::MembersCount, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count groups owned by user.
    pub async fn count_owned_by_user(&self, user_id: &str) -> AppResult<u64> {
        Group::find()
            .filter(group::Column::OwnerId.eq(user_id))
            .filter(group::Column::IsArchived.eq(false))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new group.
    pub async fn create(&self, model: group::ActiveModel) -> AppResult<group::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a group.
    pub async fn update(&self, model: group::ActiveModel) -> AppResult<group::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Archive a group (soft delete).
    pub async fn archive(&self, id: &str) -> AppResult<group::Model> {
        let group = self.get_by_id(id).await?;
        let mut active: group::ActiveModel = group.into();
        active.is_archived = Set(true);
        active.updated_at = Set(Some(Utc::now().into()));

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a group permanently.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        Group::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Increment members count atomically.
    pub async fn increment_members_count(&self, id: &str) -> AppResult<()> {
        use sea_orm::sea_query::Expr;

        Group::update_many()
            .col_expr(
                group::Column::MembersCount,
                Expr::col(group::Column::MembersCount).add(1),
            )
            .filter(group::Column::Id.eq(id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Decrement members count atomically.
    pub async fn decrement_members_count(&self, id: &str) -> AppResult<()> {
        use sea_orm::sea_query::Expr;

        Group::update_many()
            .col_expr(
                group::Column::MembersCount,
                Expr::cust("GREATEST(members_count - 1, 0)"),
            )
            .filter(group::Column::Id.eq(id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    // ==================== Member Operations ====================

    /// Check if user is a member of a group.
    pub async fn is_member(&self, user_id: &str, group_id: &str) -> AppResult<bool> {
        let count = GroupMember::find()
            .filter(group_member::Column::UserId.eq(user_id))
            .filter(group_member::Column::GroupId.eq(group_id))
            .filter(group_member::Column::IsBanned.eq(false))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count > 0)
    }

    /// Get member record.
    pub async fn get_member(
        &self,
        user_id: &str,
        group_id: &str,
    ) -> AppResult<Option<group_member::Model>> {
        GroupMember::find()
            .filter(group_member::Column::UserId.eq(user_id))
            .filter(group_member::Column::GroupId.eq(group_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Add a member to a group.
    pub async fn add_member(
        &self,
        model: group_member::ActiveModel,
    ) -> AppResult<group_member::Model> {
        let group_id = model.group_id.clone().unwrap();

        let member = model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Increment members count
        self.increment_members_count(&group_id).await?;

        Ok(member)
    }

    /// Remove a member from a group.
    pub async fn remove_member(&self, user_id: &str, group_id: &str) -> AppResult<()> {
        let deleted = GroupMember::delete_many()
            .filter(group_member::Column::UserId.eq(user_id))
            .filter(group_member::Column::GroupId.eq(group_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if deleted.rows_affected > 0 {
            self.decrement_members_count(group_id).await?;
        }

        Ok(())
    }

    /// Update member record.
    pub async fn update_member(
        &self,
        model: group_member::ActiveModel,
    ) -> AppResult<group_member::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// List members of a group.
    pub async fn list_members(
        &self,
        group_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<group_member::Model>> {
        GroupMember::find()
            .filter(group_member::Column::GroupId.eq(group_id))
            .filter(group_member::Column::IsBanned.eq(false))
            .order_by(group_member::Column::JoinedAt, Order::Asc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count members in a group.
    pub async fn count_members(&self, group_id: &str) -> AppResult<u64> {
        GroupMember::find()
            .filter(group_member::Column::GroupId.eq(group_id))
            .filter(group_member::Column::IsBanned.eq(false))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get member role.
    pub async fn get_member_role(
        &self,
        user_id: &str,
        group_id: &str,
    ) -> AppResult<Option<GroupRole>> {
        let member = self.get_member(user_id, group_id).await?;
        Ok(member.map(|m| m.role))
    }

    /// Update member role.
    pub async fn update_member_role(
        &self,
        user_id: &str,
        group_id: &str,
        role: GroupRole,
    ) -> AppResult<group_member::Model> {
        let member = self
            .get_member(user_id, group_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Member not found".to_string()))?;

        let mut active: group_member::ActiveModel = member.into();
        active.role = Set(role);
        active.updated_at = Set(Some(Utc::now().into()));

        self.update_member(active).await
    }

    // ==================== Invite Operations ====================

    /// Create an invitation or join request.
    pub async fn create_invite(
        &self,
        model: group_invite::ActiveModel,
    ) -> AppResult<group_invite::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get pending invitation for a user to a group.
    pub async fn get_pending_invite(
        &self,
        user_id: &str,
        group_id: &str,
    ) -> AppResult<Option<group_invite::Model>> {
        GroupInvite::find()
            .filter(group_invite::Column::UserId.eq(user_id))
            .filter(group_invite::Column::GroupId.eq(group_id))
            .filter(group_invite::Column::Status.eq(InviteStatus::Pending))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get invite by ID.
    pub async fn get_invite_by_id(&self, id: &str) -> AppResult<Option<group_invite::Model>> {
        GroupInvite::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update invite status.
    pub async fn update_invite_status(
        &self,
        id: &str,
        status: InviteStatus,
    ) -> AppResult<group_invite::Model> {
        let invite = self
            .get_invite_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invite not found".to_string()))?;

        let mut active: group_invite::ActiveModel = invite.into();
        active.status = Set(status);
        active.updated_at = Set(Some(Utc::now().into()));

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// List pending invites for a user.
    pub async fn list_pending_invites_for_user(
        &self,
        user_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<group_invite::Model>> {
        GroupInvite::find()
            .filter(group_invite::Column::UserId.eq(user_id))
            .filter(group_invite::Column::Status.eq(InviteStatus::Pending))
            .filter(group_invite::Column::InviteType.eq(InviteType::Invite))
            .order_by(group_invite::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// List pending join requests for a group.
    pub async fn list_pending_requests_for_group(
        &self,
        group_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<group_invite::Model>> {
        GroupInvite::find()
            .filter(group_invite::Column::GroupId.eq(group_id))
            .filter(group_invite::Column::Status.eq(InviteStatus::Pending))
            .filter(group_invite::Column::InviteType.eq(InviteType::Request))
            .order_by(group_invite::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete an invite.
    pub async fn delete_invite(&self, id: &str) -> AppResult<()> {
        GroupInvite::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::entities::group::GroupJoinPolicy;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};

    fn create_test_group(id: &str, owner_id: &str, name: &str) -> group::Model {
        group::Model {
            id: id.to_string(),
            owner_id: owner_id.to_string(),
            name: name.to_string(),
            description: None,
            banner_id: None,
            avatar_id: None,
            join_policy: GroupJoinPolicy::InviteOnly,
            is_archived: false,
            is_searchable: true,
            members_only_post: true,
            members_count: 1,
            notes_count: 0,
            rules: None,
            metadata: None,
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let group = create_test_group("grp1", "user1", "My Group");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[group.clone()]])
                .into_connection(),
        );

        let repo = GroupRepository::new(db);
        let result = repo.find_by_id("grp1").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "My Group");
    }

    #[tokio::test]
    async fn test_find_owned_by_user() {
        let grp1 = create_test_group("grp1", "user1", "Group 1");
        let grp2 = create_test_group("grp2", "user1", "Group 2");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[grp1, grp2]])
                .into_connection(),
        );

        let repo = GroupRepository::new(db);
        let result = repo.find_owned_by_user("user1", 10, 0).await.unwrap();

        assert_eq!(result.len(), 2);
    }
}
