//! Drive file repository.

use std::sync::Arc;

use crate::entities::{DriveFile, drive_file};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

/// Drive file repository for database operations.
#[derive(Clone)]
pub struct DriveFileRepository {
    db: Arc<DatabaseConnection>,
}

impl DriveFileRepository {
    /// Create a new drive file repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a file by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<drive_file::Model>> {
        DriveFile::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a file by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<drive_file::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("DriveFile: {id}")))
    }

    /// Find files by IDs.
    pub async fn find_by_ids(&self, ids: &[String]) -> AppResult<Vec<drive_file::Model>> {
        DriveFile::find()
            .filter(drive_file::Column::Id.is_in(ids.to_vec()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a file by MD5 hash.
    pub async fn find_by_md5(&self, md5: &str) -> AppResult<Option<drive_file::Model>> {
        DriveFile::find()
            .filter(drive_file::Column::Md5.eq(md5))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a file by URI.
    pub async fn find_by_uri(&self, uri: &str) -> AppResult<Option<drive_file::Model>> {
        DriveFile::find()
            .filter(drive_file::Column::Uri.eq(uri))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new file.
    pub async fn create(&self, model: drive_file::ActiveModel) -> AppResult<drive_file::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a file.
    pub async fn update(&self, model: drive_file::ActiveModel) -> AppResult<drive_file::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a file.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let file = self.find_by_id(id).await?;
        if let Some(f) = file {
            f.delete(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    /// Get files for a user (paginated).
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
        folder_id: Option<&str>,
    ) -> AppResult<Vec<drive_file::Model>> {
        let mut query = DriveFile::find()
            .filter(drive_file::Column::UserId.eq(user_id))
            .order_by_desc(drive_file::Column::Id);

        if let Some(id) = until_id {
            query = query.filter(drive_file::Column::Id.lt(id));
        }

        if let Some(fid) = folder_id {
            query = query.filter(drive_file::Column::FolderId.eq(fid));
        } else {
            query = query.filter(drive_file::Column::FolderId.is_null());
        }

        query
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Calculate total storage used by a user.
    pub async fn get_storage_used(&self, user_id: &str) -> AppResult<i64> {
        use sea_orm::FromQueryResult;

        #[derive(FromQueryResult)]
        struct SumResult {
            total: Option<i64>,
        }

        let result = DriveFile::find()
            .filter(drive_file::Column::UserId.eq(user_id))
            .filter(drive_file::Column::IsLink.eq(false))
            .select_only()
            .column_as(drive_file::Column::Size.sum(), "total")
            .into_model::<SumResult>()
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.and_then(|r| r.total).unwrap_or(0))
    }

    /// Find unattached files for a user (files not used in notes, pages, etc.)
    /// Returns files that are not referenced anywhere.
    pub async fn find_unattached(
        &self,
        user_id: &str,
        limit: u64,
    ) -> AppResult<Vec<drive_file::Model>> {
        use sea_orm::{ConnectionTrait, Statement};

        // Use raw SQL to find files not referenced in any JSON arrays
        // This checks: note.file_ids, page.file_ids, scheduled_note.file_ids
        // Also excludes files used as avatars/banners
        let sql = r#"
            SELECT df.* FROM drive_file df
            WHERE df.user_id = $1
            AND df.is_link = false
            AND NOT EXISTS (
                SELECT 1 FROM note n
                WHERE n.user_id = df.user_id
                AND n.file_ids::jsonb ? df.id
            )
            AND NOT EXISTS (
                SELECT 1 FROM page p
                WHERE p.user_id = df.user_id
                AND p.file_ids::jsonb ? df.id
            )
            AND NOT EXISTS (
                SELECT 1 FROM scheduled_note sn
                WHERE sn.user_id = df.user_id
                AND sn.file_ids::jsonb ? df.id
            )
            AND NOT EXISTS (
                SELECT 1 FROM "user" u
                WHERE u.id = df.user_id
                AND (u.avatar_url = df.url OR u.banner_url = df.url)
            )
            AND NOT EXISTS (
                SELECT 1 FROM page p2
                WHERE p2.user_id = df.user_id
                AND p2.eyecatch_image_id = df.id
            )
            ORDER BY df.created_at DESC
            LIMIT $2
        "#;

        let result = self
            .db
            .query_all(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                [user_id.into(), (limit as i64).into()],
            ))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut files = Vec::new();
        for row in result {
            use sea_orm::TryGetable;
            let file = drive_file::Model {
                id: row.try_get("", "id").unwrap_or_default(),
                user_id: row.try_get("", "user_id").unwrap_or_default(),
                user_host: row.try_get("", "user_host").ok(),
                name: row.try_get("", "name").unwrap_or_default(),
                content_type: row.try_get("", "content_type").unwrap_or_default(),
                size: row.try_get("", "size").unwrap_or(0),
                url: row.try_get("", "url").unwrap_or_default(),
                thumbnail_url: row.try_get("", "thumbnail_url").ok(),
                webpublic_url: row.try_get("", "webpublic_url").ok(),
                blurhash: row.try_get("", "blurhash").ok(),
                width: row.try_get("", "width").ok(),
                height: row.try_get("", "height").ok(),
                comment: row.try_get("", "comment").ok(),
                is_sensitive: row.try_get("", "is_sensitive").unwrap_or(false),
                is_link: row.try_get("", "is_link").unwrap_or(false),
                md5: row.try_get("", "md5").ok(),
                storage_key: row.try_get("", "storage_key").ok(),
                folder_id: row.try_get("", "folder_id").ok(),
                uri: row.try_get("", "uri").ok(),
                created_at: row
                    .try_get("", "created_at")
                    .unwrap_or_else(|_| chrono::Utc::now().into()),
            };
            files.push(file);
        }

        Ok(files)
    }

    /// Count unattached files for a user.
    pub async fn count_unattached(&self, user_id: &str) -> AppResult<i64> {
        use sea_orm::{ConnectionTrait, Statement};

        let sql = r#"
            SELECT COUNT(*) as count FROM drive_file df
            WHERE df.user_id = $1
            AND df.is_link = false
            AND NOT EXISTS (
                SELECT 1 FROM note n
                WHERE n.user_id = df.user_id
                AND n.file_ids::jsonb ? df.id
            )
            AND NOT EXISTS (
                SELECT 1 FROM page p
                WHERE p.user_id = df.user_id
                AND p.file_ids::jsonb ? df.id
            )
            AND NOT EXISTS (
                SELECT 1 FROM scheduled_note sn
                WHERE sn.user_id = df.user_id
                AND sn.file_ids::jsonb ? df.id
            )
            AND NOT EXISTS (
                SELECT 1 FROM "user" u
                WHERE u.id = df.user_id
                AND (u.avatar_url = df.url OR u.banner_url = df.url)
            )
            AND NOT EXISTS (
                SELECT 1 FROM page p2
                WHERE p2.user_id = df.user_id
                AND p2.eyecatch_image_id = df.id
            )
        "#;

        let result = self
            .db
            .query_one(Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                [user_id.into()],
            ))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(row) = result {
            use sea_orm::TryGetable;
            let count: i64 = row.try_get("", "count").unwrap_or(0);
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Delete multiple files by IDs.
    pub async fn delete_many(&self, ids: &[String]) -> AppResult<u64> {
        use sea_orm::DeleteMany;

        let result = DriveFile::delete_many()
            .filter(drive_file::Column::Id.is_in(ids.to_vec()))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected)
    }
}
