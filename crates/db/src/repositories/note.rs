//! Note repository.

use std::sync::Arc;

use crate::entities::{Note, NoteEdit, note, note_edit};
use misskey_common::{AppError, AppResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbBackend, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Statement, sea_query::Expr,
};

/// Note repository for database operations.
#[derive(Clone)]
pub struct NoteRepository {
    db: Arc<DatabaseConnection>,
}

impl NoteRepository {
    /// Create a new note repository.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Find a note by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<note::Model>> {
        Note::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a note by ID, returning an error if not found.
    pub async fn get_by_id(&self, id: &str) -> AppResult<note::Model> {
        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NoteNotFound(id.to_string()))
    }

    /// Find a note by `ActivityPub` URI.
    pub async fn find_by_uri(&self, uri: &str) -> AppResult<Option<note::Model>> {
        Note::find()
            .filter(note::Column::Uri.eq(uri))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find notes by IDs.
    pub async fn find_by_ids(&self, ids: &[String]) -> AppResult<Vec<note::Model>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        Note::find()
            .filter(note::Column::Id.is_in(ids.to_vec()))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new note.
    pub async fn create(&self, model: note::ActiveModel) -> AppResult<note::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update a note.
    pub async fn update(&self, model: note::ActiveModel) -> AppResult<note::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Delete a note.
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        Note::delete_by_id(id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    // ==================== Note Edit History ====================

    /// Create a note edit record.
    pub async fn create_edit_history(
        &self,
        model: note_edit::ActiveModel,
    ) -> AppResult<note_edit::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get edit history for a note (newest first).
    pub async fn get_edit_history(
        &self,
        note_id: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<note_edit::Model>> {
        NoteEdit::find()
            .filter(note_edit::Column::NoteId.eq(note_id))
            .order_by_desc(note_edit::Column::EditedAt)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count edit history for a note.
    pub async fn count_edit_history(&self, note_id: &str) -> AppResult<u64> {
        NoteEdit::find()
            .filter(note_edit::Column::NoteId.eq(note_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get notes by user (paginated, newest first).
    pub async fn find_by_user(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        let mut query = Note::find()
            .filter(note::Column::UserId.eq(user_id))
            .order_by_desc(note::Column::Id)
            .limit(limit);

        if let Some(until) = until_id {
            query = query.filter(note::Column::Id.lt(until));
        }

        query
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get public notes by user (for `ActivityPub` outbox).
    pub async fn find_public_by_user(
        &self,
        user_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        use sea_orm::Condition;

        let mut condition = Condition::all().add(note::Column::UserId.eq(user_id)).add(
            Condition::any()
                .add(note::Column::Visibility.eq(note::Visibility::Public))
                .add(note::Column::Visibility.eq(note::Visibility::Home)),
        );

        if let Some(until) = until_id {
            condition = condition.add(note::Column::Id.lt(until));
        }

        Note::find()
            .filter(condition)
            .order_by_desc(note::Column::Id)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get public timeline (local notes only).
    ///
    /// # Arguments
    /// * `limit` - Maximum number of notes to return
    /// * `until_id` - Return notes older than this ID (for pagination)
    /// * `exclude_user_ids` - Optional list of user IDs to exclude (for bot filtering)
    pub async fn find_local_public(
        &self,
        limit: u64,
        until_id: Option<&str>,
        exclude_user_ids: Option<&[String]>,
    ) -> AppResult<Vec<note::Model>> {
        use sea_orm::Condition;

        let mut condition = Condition::all()
            .add(note::Column::Visibility.eq(note::Visibility::Public))
            .add(note::Column::IsLocal.eq(true));

        if let Some(until) = until_id {
            condition = condition.add(note::Column::Id.lt(until));
        }

        // Exclude specified user IDs (for bot filtering)
        if let Some(user_ids) = exclude_user_ids
            && !user_ids.is_empty()
        {
            condition = condition.add(note::Column::UserId.is_not_in(user_ids.to_vec()));
        }

        Note::find()
            .filter(condition)
            .order_by_desc(note::Column::Id)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get global timeline (all public notes).
    ///
    /// # Arguments
    /// * `limit` - Maximum number of notes to return
    /// * `until_id` - Return notes older than this ID (for pagination)
    /// * `exclude_user_ids` - Optional list of user IDs to exclude (for bot filtering)
    pub async fn find_global_public(
        &self,
        limit: u64,
        until_id: Option<&str>,
        exclude_user_ids: Option<&[String]>,
    ) -> AppResult<Vec<note::Model>> {
        use sea_orm::Condition;

        let mut condition =
            Condition::all().add(note::Column::Visibility.eq(note::Visibility::Public));

        if let Some(until) = until_id {
            condition = condition.add(note::Column::Id.lt(until));
        }

        // Exclude specified user IDs (for bot filtering)
        if let Some(user_ids) = exclude_user_ids
            && !user_ids.is_empty()
        {
            condition = condition.add(note::Column::UserId.is_not_in(user_ids.to_vec()));
        }

        Note::find()
            .filter(condition)
            .order_by_desc(note::Column::Id)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get bubble timeline (local + whitelisted instances).
    ///
    /// Shows public notes from:
    /// - Local users (`user_host` IS NULL / `is_local` = true)
    /// - Users from whitelisted remote instances (`user_host` IN `bubble_hosts`)
    ///
    /// This is useful for creating a "trusted network" timeline between
    /// friendly instances.
    ///
    /// # Arguments
    /// * `bubble_hosts` - List of whitelisted instance hosts
    /// * `limit` - Maximum number of notes to return
    /// * `until_id` - Return notes older than this ID (for pagination)
    /// * `exclude_user_ids` - Optional list of user IDs to exclude (for bot filtering)
    pub async fn find_bubble_timeline(
        &self,
        bubble_hosts: &[String],
        limit: u64,
        until_id: Option<&str>,
        exclude_user_ids: Option<&[String]>,
    ) -> AppResult<Vec<note::Model>> {
        use sea_orm::Condition;

        // Build condition: Public AND (local OR from whitelisted hosts)
        let mut host_condition = Condition::any().add(note::Column::IsLocal.eq(true));

        // Add whitelisted hosts to the condition
        if !bubble_hosts.is_empty() {
            host_condition =
                host_condition.add(note::Column::UserHost.is_in(bubble_hosts.to_vec()));
        }

        let mut condition = Condition::all()
            .add(note::Column::Visibility.eq(note::Visibility::Public))
            .add(host_condition);

        if let Some(until) = until_id {
            condition = condition.add(note::Column::Id.lt(until));
        }

        // Exclude specified user IDs (for bot filtering)
        if let Some(user_ids) = exclude_user_ids
            && !user_ids.is_empty()
        {
            condition = condition.add(note::Column::UserId.is_not_in(user_ids.to_vec()));
        }

        Note::find()
            .filter(condition)
            .order_by_desc(note::Column::Id)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get replies to a note.
    pub async fn find_replies(&self, note_id: &str, limit: u64) -> AppResult<Vec<note::Model>> {
        Note::find()
            .filter(note::Column::ReplyId.eq(note_id))
            .order_by_asc(note::Column::Id)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get renotes of a note.
    pub async fn find_renotes(&self, note_id: &str, limit: u64) -> AppResult<Vec<note::Model>> {
        Note::find()
            .filter(note::Column::RenoteId.eq(note_id))
            .filter(note::Column::Text.is_null()) // Pure renotes only
            .order_by_desc(note::Column::Id)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find a renote by a specific user.
    pub async fn find_renote(
        &self,
        user_id: &str,
        note_id: &str,
    ) -> AppResult<Option<note::Model>> {
        Note::find()
            .filter(note::Column::UserId.eq(user_id))
            .filter(note::Column::RenoteId.eq(note_id))
            .filter(note::Column::Text.is_null()) // Pure renotes only
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get home timeline (notes from followed users + own notes).
    ///
    /// # Arguments
    /// * `user_id` - The user's ID
    /// * `following_ids` - List of user IDs the user is following
    /// * `limit` - Maximum number of notes to return
    /// * `until_id` - Return notes older than this ID (for pagination)
    /// * `exclude_user_ids` - Optional list of user IDs to exclude (for bot filtering)
    pub async fn find_home_timeline(
        &self,
        user_id: &str,
        following_ids: &[String],
        limit: u64,
        until_id: Option<&str>,
        exclude_user_ids: Option<&[String]>,
    ) -> AppResult<Vec<note::Model>> {
        use sea_orm::Condition;

        // Include own notes and notes from followed users
        let mut user_ids = following_ids.to_vec();
        user_ids.push(user_id.to_string());

        // If we have user IDs to exclude, remove them from the source list
        // (more efficient than NOT IN for followed users)
        if let Some(exclude_ids) = exclude_user_ids {
            user_ids.retain(|id| !exclude_ids.contains(id));
        }

        let mut condition = Condition::all()
            .add(note::Column::UserId.is_in(user_ids))
            .add(
                Condition::any()
                    .add(note::Column::Visibility.eq(note::Visibility::Public))
                    .add(note::Column::Visibility.eq(note::Visibility::Home))
                    .add(note::Column::Visibility.eq(note::Visibility::Followers)),
            );

        if let Some(until) = until_id {
            condition = condition.add(note::Column::Id.lt(until));
        }

        Note::find()
            .filter(condition)
            .order_by_desc(note::Column::Id)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Increment reaction count atomically (single UPDATE query, no fetch).
    pub async fn increment_reactions_count(&self, note_id: &str) -> AppResult<()> {
        Note::update_many()
            .col_expr(
                note::Column::ReactionCount,
                Expr::col(note::Column::ReactionCount).add(1),
            )
            .filter(note::Column::Id.eq(note_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Decrement reaction count atomically (single UPDATE query, no fetch).
    pub async fn decrement_reactions_count(&self, note_id: &str) -> AppResult<()> {
        Note::update_many()
            .col_expr(
                note::Column::ReactionCount,
                Expr::cust("GREATEST(reaction_count - 1, 0)"),
            )
            .filter(note::Column::Id.eq(note_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Increment replies count atomically (single UPDATE query, no fetch).
    pub async fn increment_replies_count(&self, note_id: &str) -> AppResult<()> {
        Note::update_many()
            .col_expr(
                note::Column::RepliesCount,
                Expr::col(note::Column::RepliesCount).add(1),
            )
            .filter(note::Column::Id.eq(note_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Decrement replies count atomically (single UPDATE query, no fetch).
    pub async fn decrement_replies_count(&self, note_id: &str) -> AppResult<()> {
        Note::update_many()
            .col_expr(
                note::Column::RepliesCount,
                Expr::cust("GREATEST(replies_count - 1, 0)"),
            )
            .filter(note::Column::Id.eq(note_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Increment renote count atomically (single UPDATE query, no fetch).
    pub async fn increment_renote_count(&self, note_id: &str) -> AppResult<()> {
        Note::update_many()
            .col_expr(
                note::Column::RenoteCount,
                Expr::col(note::Column::RenoteCount).add(1),
            )
            .filter(note::Column::Id.eq(note_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Decrement renote count atomically (single UPDATE query, no fetch).
    pub async fn decrement_renote_count(&self, note_id: &str) -> AppResult<()> {
        Note::update_many()
            .col_expr(
                note::Column::RenoteCount,
                Expr::cust("GREATEST(renote_count - 1, 0)"),
            )
            .filter(note::Column::Id.eq(note_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Search notes by text content using `PostgreSQL` full-text search.
    /// Falls back to LIKE if full-text search fails.
    pub async fn search(
        &self,
        query: &str,
        limit: u64,
        until_id: Option<&str>,
        user_id: Option<&str>,
        host: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        // Try full-text search first
        match self
            .search_fulltext(query, limit, until_id, user_id, host)
            .await
        {
            Ok(results) => Ok(results),
            Err(_) => {
                // Fallback to LIKE search
                self.search_like(query, limit, until_id, user_id, host)
                    .await
            }
        }
    }

    /// Full-text search using `PostgreSQL` tsvector/tsquery.
    /// Uses GIN index for efficient searching.
    pub async fn search_fulltext(
        &self,
        query: &str,
        limit: u64,
        until_id: Option<&str>,
        user_id: Option<&str>,
        host: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        // Escape query for tsquery
        let escaped_query = query
            .replace('\\', "\\\\")
            .replace('\'', "''")
            .replace(['&', '|', '!', '(', ')', ':'], " ");

        // Build WHERE clause conditions
        let mut conditions = vec!["visibility = 'Public'".to_string()];

        if let Some(until) = until_id {
            conditions.push(format!("id < '{}'", until.replace('\'', "''")));
        }

        if let Some(uid) = user_id {
            conditions.push(format!("user_id = '{}'", uid.replace('\'', "''")));
        }

        if let Some(h) = host {
            if h.is_empty() {
                conditions.push("user_host IS NULL".to_string());
            } else {
                conditions.push(format!("user_host = '{}'", h.replace('\'', "''")));
            }
        }

        let where_clause = conditions.join(" AND ");

        // Full-text search query with relevance ranking
        let sql = format!(
            r"
            SELECT
                id, user_id, user_host, text, cw, visibility,
                reply_id, renote_id, thread_id, mentions, visible_user_ids,
                file_ids, tags, reactions, replies_count, renote_count,
                reaction_count, is_local, uri, url, created_at, updated_at
            FROM note
            WHERE {where_clause}
                AND to_tsvector('simple', COALESCE(text, '')) @@ plainto_tsquery('simple', $1)
            ORDER BY
                ts_rank(to_tsvector('simple', COALESCE(text, '')), plainto_tsquery('simple', $1)) DESC,
                created_at DESC
            LIMIT $2
            "
        );

        Note::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &sql,
                [escaped_query.into(), (limit as i64).into()],
            ))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Fallback LIKE-based search for when full-text search is unavailable.
    pub async fn search_like(
        &self,
        query: &str,
        limit: u64,
        until_id: Option<&str>,
        user_id: Option<&str>,
        host: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        use sea_orm::Condition;

        let search_pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));

        let mut condition = Condition::all()
            .add(note::Column::Text.like(&search_pattern))
            .add(note::Column::Visibility.eq(note::Visibility::Public));

        if let Some(until) = until_id {
            condition = condition.add(note::Column::Id.lt(until));
        }

        if let Some(uid) = user_id {
            condition = condition.add(note::Column::UserId.eq(uid));
        }

        if let Some(h) = host {
            if h.is_empty() {
                condition = condition.add(note::Column::UserHost.is_null());
            } else {
                condition = condition.add(note::Column::UserHost.eq(h));
            }
        }

        Note::find()
            .filter(condition)
            .order_by_desc(note::Column::Id)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Search trending notes (high reaction count).
    pub async fn find_trending(
        &self,
        limit: u64,
        min_reactions: i32,
        hours: i64,
    ) -> AppResult<Vec<note::Model>> {
        let since = chrono::Utc::now() - chrono::Duration::hours(hours);

        let sql = r"
            SELECT
                id, user_id, user_host, text, cw, visibility,
                reply_id, renote_id, thread_id, mentions, visible_user_ids,
                file_ids, tags, reactions, replies_count, renote_count,
                reaction_count, is_local, uri, url, created_at, updated_at
            FROM note
            WHERE visibility = 'Public'
                AND reaction_count >= $1
                AND created_at >= $2
            ORDER BY reaction_count DESC, created_at DESC
            LIMIT $3
        ";

        Note::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [min_reactions.into(), since.into(), (limit as i64).into()],
            ))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get thread (conversation) for a note.
    /// Returns all notes in the thread, ordered by creation time.
    pub async fn find_thread(&self, thread_id: &str, limit: u64) -> AppResult<Vec<note::Model>> {
        Note::find()
            .filter(note::Column::ThreadId.eq(thread_id))
            .order_by_asc(note::Column::Id)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get the conversation chain for a note (all ancestors up to root).
    /// Uses recursive CTE for O(1) query instead of O(N) loop.
    pub async fn find_ancestors(&self, note_id: &str, limit: usize) -> AppResult<Vec<note::Model>> {
        // Use PostgreSQL recursive CTE for efficient ancestor traversal
        let sql = r"
            WITH RECURSIVE ancestors AS (
                -- Base case: start with the given note
                SELECT n.*, 0 as depth
                FROM note n
                WHERE n.id = $1

                UNION ALL

                -- Recursive case: get parent notes
                SELECT n.*, a.depth + 1
                FROM note n
                INNER JOIN ancestors a ON n.id = a.reply_id
                WHERE a.depth < $2
            )
            SELECT
                id, user_id, user_host, text, cw, visibility,
                reply_id, renote_id, thread_id, mentions, visible_user_ids,
                file_ids, tags, reactions, replies_count, renote_count,
                reaction_count, is_local, uri, url, created_at, updated_at
            FROM ancestors
            WHERE id != $1
            ORDER BY depth DESC
        ";

        let notes = Note::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [note_id.into(), (limit as i64).into()],
            ))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(notes)
    }

    /// Get the conversation chain for a note (fallback for non-PostgreSQL).
    /// Uses iterative approach with N queries.
    #[allow(dead_code)]
    pub async fn find_ancestors_iterative(
        &self,
        note_id: &str,
        limit: usize,
    ) -> AppResult<Vec<note::Model>> {
        let mut ancestors = Vec::new();
        let mut current_id = Some(note_id.to_string());

        while let Some(id) = current_id {
            if ancestors.len() >= limit {
                break;
            }

            if let Some(note) = self.find_by_id(&id).await? {
                current_id = note.reply_id.clone();
                if id != note_id {
                    ancestors.push(note);
                }
            } else {
                break;
            }
        }

        // Reverse to get oldest first
        ancestors.reverse();
        Ok(ancestors)
    }

    /// Get direct descendants (immediate replies) for a note.
    pub async fn find_children(
        &self,
        note_id: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        let mut query = Note::find()
            .filter(note::Column::ReplyId.eq(note_id))
            .order_by_asc(note::Column::Id)
            .limit(limit);

        if let Some(until) = until_id {
            query = query.filter(note::Column::Id.lt(until));
        }

        query
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Search notes by hashtag.
    pub async fn search_by_tag(
        &self,
        tag: &str,
        limit: u64,
        until_id: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        use sea_orm::Condition;

        // Search in JSON tags array
        // PostgreSQL: tags @> '["tag"]'::jsonb
        let tag_lower = tag.to_lowercase();
        let tag_json = format!("[\"{tag_lower}\"]");

        let mut condition =
            Condition::all().add(note::Column::Visibility.eq(note::Visibility::Public));

        if let Some(until) = until_id {
            condition = condition.add(note::Column::Id.lt(until));
        }

        // For JSON containment, we need raw SQL
        // This is a simplified version - actual implementation might need custom query
        Note::find()
            .filter(condition)
            .filter(note::Column::Tags.contains(&tag_json))
            .order_by_desc(note::Column::Id)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count total local notes.
    pub async fn count_local_notes(&self) -> AppResult<u64> {
        Note::find()
            .filter(note::Column::IsLocal.eq(true))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    // ==================== Channel Timeline ====================

    /// Get channel timeline (notes posted to a specific channel).
    pub async fn find_by_channel(
        &self,
        channel_id: &str,
        limit: u64,
        until_id: Option<&str>,
        since_id: Option<&str>,
    ) -> AppResult<Vec<note::Model>> {
        use sea_orm::Condition;

        let mut condition = Condition::all().add(note::Column::ChannelId.eq(channel_id));

        if let Some(until) = until_id {
            condition = condition.add(note::Column::Id.lt(until));
        }

        if let Some(since) = since_id {
            condition = condition.add(note::Column::Id.gt(since));
        }

        Note::find()
            .filter(condition)
            .order_by_desc(note::Column::Id)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count notes in a channel.
    pub async fn count_by_channel(&self, channel_id: &str) -> AppResult<u64> {
        Note::find()
            .filter(note::Column::ChannelId.eq(channel_id))
            .count(self.db.as_ref())
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
    use serde_json::json;
    use std::sync::Arc;

    fn create_test_note(id: &str, user_id: &str, text: Option<&str>) -> note::Model {
        note::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            user_host: None,
            text: text.map(std::string::ToString::to_string),
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
            channel_id: None,
            created_at: Utc::now().into(),
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_find_by_id_found() {
        let note = create_test_note("note1", "user1", Some("Hello world"));

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[note.clone()]])
                .into_connection(),
        );

        let repo = NoteRepository::new(db);
        let result = repo.find_by_id("note1").await.unwrap();

        assert!(result.is_some());
        let found_note = result.unwrap();
        assert_eq!(found_note.id, "note1");
        assert_eq!(found_note.text, Some("Hello world".to_string()));
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<note::Model>::new()])
                .into_connection(),
        );

        let repo = NoteRepository::new(db);
        let result = repo.find_by_id("nonexistent").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_by_id_not_found_returns_error() {
        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([Vec::<note::Model>::new()])
                .into_connection(),
        );

        let repo = NoteRepository::new(db);
        let result = repo.get_by_id("nonexistent").await;

        assert!(result.is_err());
        match result {
            Err(AppError::NoteNotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected NoteNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_find_by_user() {
        let note1 = create_test_note("note1", "user1", Some("First note"));
        let note2 = create_test_note("note2", "user1", Some("Second note"));

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[note1, note2]])
                .into_connection(),
        );

        let repo = NoteRepository::new(db);
        let result = repo.find_by_user("user1", 10, None).await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_find_local_public() {
        let note1 = create_test_note("note1", "user1", Some("Public note"));
        let note2 = create_test_note("note2", "user2", Some("Another public note"));

        let db = Arc::new(
            MockDatabase::new(DatabaseBackend::Postgres)
                .append_query_results([[note1, note2]])
                .into_connection(),
        );

        let repo = NoteRepository::new(db);
        let result = repo.find_local_public(10, None, None).await.unwrap();

        assert_eq!(result.len(), 2);
    }
}
