//! Note favorite (bookmark) service.

use misskey_common::{AppError, AppResult, IdGenerator};
use misskey_db::{
    entities::note_favorite,
    repositories::{NoteFavoriteRepository, NoteRepository},
};
use sea_orm::Set;

/// Note favorite service for managing bookmarks.
#[derive(Clone)]
pub struct NoteFavoriteService {
    favorite_repo: NoteFavoriteRepository,
    note_repo: NoteRepository,
    id_gen: IdGenerator,
}

impl NoteFavoriteService {
    /// Create a new note favorite service.
    #[must_use] 
    pub const fn new(favorite_repo: NoteFavoriteRepository, note_repo: NoteRepository) -> Self {
        Self {
            favorite_repo,
            note_repo,
            id_gen: IdGenerator::new(),
        }
    }

    /// Add a note to favorites (bookmark).
    pub async fn create(&self, user_id: &str, note_id: &str) -> AppResult<note_favorite::Model> {
        // Check if note exists
        self.note_repo.get_by_id(note_id).await?;

        // Check if already favorited
        if self.favorite_repo.is_favorited(user_id, note_id).await? {
            return Err(AppError::BadRequest("Note already favorited".to_string()));
        }

        // Create favorite
        let id = self.id_gen.generate();
        let model = note_favorite::ActiveModel {
            id: Set(id),
            user_id: Set(user_id.to_string()),
            note_id: Set(note_id.to_string()),
            created_at: Set(chrono::Utc::now().into()),
        };

        self.favorite_repo.create(model).await
    }

    /// Remove a note from favorites (unbookmark).
    pub async fn delete(&self, user_id: &str, note_id: &str) -> AppResult<()> {
        // Check if favorited
        if !self.favorite_repo.is_favorited(user_id, note_id).await? {
            return Err(AppError::NotFound("Favorite not found".to_string()));
        }

        self.favorite_repo
            .delete_by_user_and_note(user_id, note_id)
            .await
    }

    /// Check if a note is favorited by user.
    pub async fn is_favorited(&self, user_id: &str, note_id: &str) -> AppResult<bool> {
        self.favorite_repo.is_favorited(user_id, note_id).await
    }

    /// Get user's favorites (paginated).
    pub async fn get_favorites(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<note_favorite::Model>> {
        self.favorite_repo.find_by_user(user_id, limit, until_id).await
    }

    /// Count user's favorites.
    pub async fn count(&self, user_id: &str) -> AppResult<u64> {
        self.favorite_repo.count_by_user(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use misskey_db::entities::note;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use serde_json::json;
    use std::sync::Arc;

    fn create_test_note(id: &str) -> note::Model {
        note::Model {
            id: id.to_string(),
            user_id: "user1".to_string(),
            user_host: None,
            text: Some("Test note".to_string()),
            cw: None,
            visibility: note::Visibility::Public,
            reply_id: None,
            renote_id: None,
            thread_id: None,
            mentions: json!([]),
            visible_user_ids: json!([]),
            file_ids: json!([]),
            tags: json!([]),
            reactions: json!({}),
            replies_count: 0,
            renote_count: 0,
            reaction_count: 0,
            is_local: true,
            uri: None,
            url: None,
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    fn create_test_favorite(id: &str, user_id: &str, note_id: &str) -> note_favorite::Model {
        note_favorite::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            note_id: note_id.to_string(),
            created_at: Utc::now().into(),
        }
    }

    #[tokio::test]
    async fn test_is_favorited_true() {
        let fav = create_test_favorite("fav1", "user1", "note1");

        let fav_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[fav]])
                .into_connection(),
        );
        let note_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let fav_repo = NoteFavoriteRepository::new(fav_db);
        let note_repo = NoteRepository::new(note_db);
        let service = NoteFavoriteService::new(fav_repo, note_repo);

        let result = service.is_favorited("user1", "note1").await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_is_favorited_false() {
        let fav_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<note_favorite::Model>::new()])
                .into_connection(),
        );
        let note_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let fav_repo = NoteFavoriteRepository::new(fav_db);
        let note_repo = NoteRepository::new(note_db);
        let service = NoteFavoriteService::new(fav_repo, note_repo);

        let result = service.is_favorited("user1", "note1").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_create_already_favorited() {
        let note = create_test_note("note1");
        let fav = create_test_favorite("fav1", "user1", "note1");

        let fav_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[fav]])
                .into_connection(),
        );
        let note_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[note]])
                .into_connection(),
        );

        let fav_repo = NoteFavoriteRepository::new(fav_db);
        let note_repo = NoteRepository::new(note_db);
        let service = NoteFavoriteService::new(fav_repo, note_repo);

        let result = service.create("user1", "note1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_not_favorited() {
        let fav_db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<note_favorite::Model>::new()])
                .into_connection(),
        );
        let note_db = Arc::new(MockDatabase::new(DatabaseBackend::Postgres).into_connection());

        let fav_repo = NoteFavoriteRepository::new(fav_db);
        let note_repo = NoteRepository::new(note_db);
        let service = NoteFavoriteService::new(fav_repo, note_repo);

        let result = service.delete("user1", "note1").await;
        assert!(result.is_err());
    }
}
