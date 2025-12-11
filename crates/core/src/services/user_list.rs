//! User list service.

use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::{user_list, user_list_member},
    repositories::{UserListRepository, UserRepository},
};
use sea_orm::Set;

/// User list service for managing user lists.
#[derive(Clone)]
pub struct UserListService {
    list_repo: UserListRepository,
    user_repo: UserRepository,
    id_gen: IdGenerator,
}

/// Input for creating a list.
pub struct CreateListInput {
    pub name: String,
    pub is_public: bool,
}

impl UserListService {
    /// Create a new user list service.
    #[must_use]
    pub const fn new(list_repo: UserListRepository, user_repo: UserRepository) -> Self {
        Self {
            list_repo,
            user_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Create a new list.
    pub async fn create(
        &self,
        user_id: &str,
        input: CreateListInput,
    ) -> AppResult<user_list::Model> {
        // Validate name
        let name = input.name.trim();
        if name.is_empty() {
            return Err(AppError::BadRequest("List name is required".to_string()));
        }
        if name.len() > 100 {
            return Err(AppError::BadRequest("List name too long".to_string()));
        }

        let id = self.id_gen.generate();
        let model = user_list::ActiveModel {
            id: Set(id),
            user_id: Set(user_id.to_string()),
            name: Set(name.to_string()),
            is_public: Set(input.is_public),
            created_at: Set(chrono::Utc::now().into()),
        };

        self.list_repo.create(model).await
    }

    /// Get a list by ID.
    pub async fn get(&self, id: &str) -> AppResult<user_list::Model> {
        self.list_repo.get_by_id(id).await
    }

    /// Update a list.
    pub async fn update(
        &self,
        user_id: &str,
        list_id: &str,
        name: Option<String>,
        is_public: Option<bool>,
    ) -> AppResult<user_list::Model> {
        let list = self.list_repo.get_by_id(list_id).await?;

        // Verify ownership
        if list.user_id != user_id {
            return Err(AppError::Forbidden("Not your list".to_string()));
        }

        let mut model: user_list::ActiveModel = list.into();

        if let Some(n) = name {
            let n = n.trim();
            if n.is_empty() {
                return Err(AppError::BadRequest("List name is required".to_string()));
            }
            if n.len() > 100 {
                return Err(AppError::BadRequest("List name too long".to_string()));
            }
            model.name = Set(n.to_string());
        }

        if let Some(public) = is_public {
            model.is_public = Set(public);
        }

        self.list_repo.update(model).await
    }

    /// Delete a list.
    pub async fn delete(&self, user_id: &str, list_id: &str) -> AppResult<()> {
        let list = self.list_repo.get_by_id(list_id).await?;

        // Verify ownership
        if list.user_id != user_id {
            return Err(AppError::Forbidden("Not your list".to_string()));
        }

        self.list_repo.delete(list_id).await
    }

    /// Get lists by user.
    pub async fn get_lists(&self, user_id: &str) -> AppResult<Vec<user_list::Model>> {
        self.list_repo.find_by_user(user_id).await
    }

    /// Add a user to a list.
    pub async fn add_member(
        &self,
        owner_id: &str,
        list_id: &str,
        user_id: &str,
    ) -> AppResult<user_list_member::Model> {
        let list = self.list_repo.get_by_id(list_id).await?;

        // Verify ownership
        if list.user_id != owner_id {
            return Err(AppError::Forbidden("Not your list".to_string()));
        }

        // Check if user exists
        self.user_repo.get_by_id(user_id).await?;

        // Check if already a member
        if self.list_repo.is_member(list_id, user_id).await? {
            return Err(AppError::BadRequest("User already in list".to_string()));
        }

        let id = self.id_gen.generate();
        let model = user_list_member::ActiveModel {
            id: Set(id),
            list_id: Set(list_id.to_string()),
            user_id: Set(user_id.to_string()),
            created_at: Set(chrono::Utc::now().into()),
        };

        self.list_repo.add_member(model).await
    }

    /// Remove a user from a list.
    pub async fn remove_member(
        &self,
        owner_id: &str,
        list_id: &str,
        user_id: &str,
    ) -> AppResult<()> {
        let list = self.list_repo.get_by_id(list_id).await?;

        // Verify ownership
        if list.user_id != owner_id {
            return Err(AppError::Forbidden("Not your list".to_string()));
        }

        // Check if member
        if !self.list_repo.is_member(list_id, user_id).await? {
            return Err(AppError::NotFound("User not in list".to_string()));
        }

        self.list_repo.remove_member(list_id, user_id).await
    }

    /// Get members of a list.
    pub async fn get_members(&self, list_id: &str) -> AppResult<Vec<String>> {
        self.list_repo.find_member_ids(list_id).await
    }

    /// Check if a user can view a list.
    pub async fn can_view(&self, viewer_id: Option<&str>, list_id: &str) -> AppResult<bool> {
        let list = self.list_repo.get_by_id(list_id).await?;

        // Public lists can be viewed by anyone
        if list.is_public {
            return Ok(true);
        }

        // Private lists can only be viewed by owner
        if let Some(id) = viewer_id {
            return Ok(list.user_id == id);
        }

        Ok(false)
    }

    /// Get all list IDs that a user is a member of.
    /// Used for antenna matching to check if a note author is in any lists.
    pub async fn get_list_memberships_for_user(&self, user_id: &str) -> AppResult<Vec<String>> {
        self.list_repo.find_list_ids_for_member(user_id).await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_create_list_input() {
        let input = CreateListInput {
            name: "My List".to_string(),
            is_public: false,
        };
        assert_eq!(input.name, "My List");
        assert!(!input.is_public);
    }
}
