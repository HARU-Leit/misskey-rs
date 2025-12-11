//! OAuth repository for application and token management.

use std::sync::Arc;

use crate::entities::{OAuthApp, OAuthToken, oauth_app, oauth_token};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, sea_query::Expr,
};

/// OAuth repository for database operations.
#[derive(Clone)]
pub struct OAuthRepository {
    db: Arc<DatabaseConnection>,
}

impl OAuthRepository {
    /// Create a new OAuth repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // ==================== Application Operations ====================

    /// Find an OAuth application by ID.
    pub async fn find_app_by_id(&self, id: &str) -> AppResult<Option<oauth_app::Model>> {
        OAuthApp::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get an OAuth application by ID, returning an error if not found.
    pub async fn get_app_by_id(&self, id: &str) -> AppResult<oauth_app::Model> {
        self.find_app_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("OAuthApp: {id}")))
    }

    /// Find an OAuth application by client ID.
    pub async fn find_app_by_client_id(
        &self,
        client_id: &str,
    ) -> AppResult<Option<oauth_app::Model>> {
        OAuthApp::find()
            .filter(oauth_app::Column::ClientId.eq(client_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get an OAuth application by client ID, returning an error if not found.
    pub async fn get_app_by_client_id(&self, client_id: &str) -> AppResult<oauth_app::Model> {
        self.find_app_by_client_id(client_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("OAuthApp with client_id: {client_id}")))
    }

    /// Find all OAuth applications for a user.
    pub async fn find_apps_by_user_id(&self, user_id: &str) -> AppResult<Vec<oauth_app::Model>> {
        OAuthApp::find()
            .filter(oauth_app::Column::UserId.eq(user_id))
            .order_by_desc(oauth_app::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new OAuth application.
    pub async fn create_app(&self, model: oauth_app::ActiveModel) -> AppResult<oauth_app::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update an OAuth application.
    pub async fn update_app(&self, model: oauth_app::ActiveModel) -> AppResult<oauth_app::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete an OAuth application.
    pub async fn delete_app(&self, id: &str) -> AppResult<()> {
        OAuthApp::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    // ==================== Token Operations ====================

    /// Find a token by ID.
    pub async fn find_token_by_id(&self, id: &str) -> AppResult<Option<oauth_token::Model>> {
        OAuthToken::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a token by its hash.
    pub async fn find_token_by_hash(
        &self,
        token_hash: &str,
    ) -> AppResult<Option<oauth_token::Model>> {
        OAuthToken::find()
            .filter(oauth_token::Column::TokenHash.eq(token_hash))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find all tokens for a user.
    pub async fn find_tokens_by_user_id(
        &self,
        user_id: &str,
    ) -> AppResult<Vec<oauth_token::Model>> {
        OAuthToken::find()
            .filter(oauth_token::Column::UserId.eq(user_id))
            .filter(oauth_token::Column::IsRevoked.eq(false))
            .order_by_desc(oauth_token::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find all tokens for an application.
    pub async fn find_tokens_by_app_id(&self, app_id: &str) -> AppResult<Vec<oauth_token::Model>> {
        OAuthToken::find()
            .filter(oauth_token::Column::AppId.eq(app_id))
            .filter(oauth_token::Column::IsRevoked.eq(false))
            .order_by_desc(oauth_token::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new token.
    pub async fn create_token(
        &self,
        model: oauth_token::ActiveModel,
    ) -> AppResult<oauth_token::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a token.
    pub async fn update_token(
        &self,
        model: oauth_token::ActiveModel,
    ) -> AppResult<oauth_token::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Revoke a token by ID.
    pub async fn revoke_token(&self, id: &str) -> AppResult<()> {
        let token = self
            .find_token_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("OAuthToken: {id}")))?;

        let mut active: oauth_token::ActiveModel = token.into();
        active.is_revoked = Set(true);
        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Revoke all tokens for a user and application.
    pub async fn revoke_tokens_for_user_app(&self, user_id: &str, app_id: &str) -> AppResult<u64> {
        use sea_orm::QueryTrait;

        let result = OAuthToken::update_many()
            .col_expr(oauth_token::Column::IsRevoked, Expr::value(true))
            .filter(oauth_token::Column::UserId.eq(user_id))
            .filter(oauth_token::Column::AppId.eq(app_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected)
    }

    /// Delete expired tokens (for cleanup).
    pub async fn delete_expired_tokens(&self) -> AppResult<u64> {
        let now = chrono::Utc::now().fixed_offset();

        let result = OAuthToken::delete_many()
            .filter(oauth_token::Column::ExpiresAt.lt(now))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected)
    }

    /// Update last_used_at for a token.
    pub async fn touch_token(&self, id: &str) -> AppResult<()> {
        let token = self
            .find_token_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("OAuthToken: {id}")))?;

        let now = chrono::Utc::now().fixed_offset();
        let mut active: oauth_token::ActiveModel = token.into();
        active.last_used_at = Set(Some(now));
        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Count tokens for a user.
    pub async fn count_tokens_by_user_id(&self, user_id: &str) -> AppResult<u64> {
        OAuthToken::find()
            .filter(oauth_token::Column::UserId.eq(user_id))
            .filter(oauth_token::Column::IsRevoked.eq(false))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }
}
