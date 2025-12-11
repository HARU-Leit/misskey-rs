//! User list repository.

use std::sync::Arc;

use crate::entities::{UserList, UserListMember, user_list, user_list_member};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};

/// User list repository for database operations.
#[derive(Clone)]
pub struct UserListRepository {
    db: Arc<DatabaseConnection>,
}

impl UserListRepository {
    /// Create a new user list repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a list by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<user_list::Model>> {
        UserList::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a list by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<user_list::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("List {id} not found")))
    }

    /// Create a new list.
    pub async fn create(&self, model: user_list::ActiveModel) -> AppResult<user_list::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a list.
    pub async fn update(&self, model: user_list::ActiveModel) -> AppResult<user_list::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a list by ID.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        // Delete all members first
        UserListMember::delete_many()
            .filter(user_list_member::Column::ListId.eq(id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Delete the list
        UserList::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Get lists by user.
    pub async fn find_by_user(&self, user_id: &str) -> AppResult<Vec<user_list::Model>> {
        UserList::find()
            .filter(user_list::Column::UserId.eq(user_id))
            .order_by_desc(user_list::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Add a member to a list.
    pub async fn add_member(
        &self,
        model: user_list_member::ActiveModel,
    ) -> AppResult<user_list_member::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Remove a member from a list.
    pub async fn remove_member(&self, list_id: &str, user_id: &str) -> AppResult<()> {
        UserListMember::delete_many()
            .filter(user_list_member::Column::ListId.eq(list_id))
            .filter(user_list_member::Column::UserId.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Check if a user is a member of a list.
    pub async fn is_member(&self, list_id: &str, user_id: &str) -> AppResult<bool> {
        let member = UserListMember::find()
            .filter(user_list_member::Column::ListId.eq(list_id))
            .filter(user_list_member::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(member.is_some())
    }

    /// Get members of a list.
    pub async fn find_members(&self, list_id: &str) -> AppResult<Vec<user_list_member::Model>> {
        UserListMember::find()
            .filter(user_list_member::Column::ListId.eq(list_id))
            .order_by_asc(user_list_member::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get member IDs of a list.
    pub async fn find_member_ids(&self, list_id: &str) -> AppResult<Vec<String>> {
        let members = self.find_members(list_id).await?;
        Ok(members.into_iter().map(|m| m.user_id).collect())
    }

    /// Get lists that contain a specific user.
    pub async fn find_lists_containing_user(
        &self,
        owner_id: &str,
        member_user_id: &str,
    ) -> AppResult<Vec<user_list::Model>> {
        // Get list IDs where user is a member
        let memberships = UserListMember::find()
            .filter(user_list_member::Column::UserId.eq(member_user_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let list_ids: Vec<String> = memberships.into_iter().map(|m| m.list_id).collect();

        if list_ids.is_empty() {
            return Ok(vec![]);
        }

        // Get lists owned by owner_id that contain the user
        UserList::find()
            .filter(user_list::Column::Id.is_in(list_ids))
            .filter(user_list::Column::UserId.eq(owner_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseBackend, MockDatabase};

    fn create_test_list(id: &str, user_id: &str, name: &str) -> user_list::Model {
        user_list::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            name: name.to_string(),
            is_public: false,
            created_at: Utc::now().into(),
        }
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let list = create_test_list("list1", "user1", "My List");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[list.clone()]])
                .into_connection(),
        );

        let repo = UserListRepository::new(db);
        let result = repo.find_by_id("list1").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "My List");
    }

    #[tokio::test]
    async fn test_find_by_user() {
        let list1 = create_test_list("list1", "user1", "List 1");
        let list2 = create_test_list("list2", "user1", "List 2");

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[list1, list2]])
                .into_connection(),
        );

        let repo = UserListRepository::new(db);
        let result = repo.find_by_user("user1").await.unwrap();

        assert_eq!(result.len(), 2);
    }
}
