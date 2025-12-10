//! Add full-text search indexes for notes and users.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create GIN index for note text search
        // Uses 'simple' configuration for multi-language support
        // (for Japanese-specific, consider using 'japanese' with pg_bigm or pgroonga)
        manager
            .get_connection()
            .execute_unprepared(
                r"
                CREATE INDEX IF NOT EXISTS idx_note_text_search
                ON note
                USING GIN (to_tsvector('simple', COALESCE(text, '')))
                WHERE visibility = 'Public';
                ",
            )
            .await?;

        // Create GIN index for note CW (content warning) search
        manager
            .get_connection()
            .execute_unprepared(
                r"
                CREATE INDEX IF NOT EXISTS idx_note_cw_search
                ON note
                USING GIN (to_tsvector('simple', COALESCE(cw, '')))
                WHERE cw IS NOT NULL;
                ",
            )
            .await?;

        // Create GIN index for user search (username + display name)
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX IF NOT EXISTS idx_user_search
                ON "user"
                USING GIN (
                    to_tsvector('simple', COALESCE(username_lower, '') || ' ' || COALESCE(name, ''))
                )
                WHERE is_suspended = false;
                "#,
            )
            .await?;

        // Create index for trending notes (sorted by reaction count)
        manager
            .get_connection()
            .execute_unprepared(
                r"
                CREATE INDEX IF NOT EXISTS idx_note_trending
                ON note (reaction_count DESC, created_at DESC)
                WHERE visibility = 'Public' AND reaction_count > 0;
                ",
            )
            .await?;

        // Create index for popular users (sorted by followers count)
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE INDEX IF NOT EXISTS idx_user_popular
                ON "user" (followers_count DESC)
                WHERE is_suspended = false AND host IS NULL;
                "#,
            )
            .await?;

        // Create index for note created_at + visibility (common timeline filter)
        manager
            .get_connection()
            .execute_unprepared(
                r"
                CREATE INDEX IF NOT EXISTS idx_note_timeline_public
                ON note (created_at DESC)
                WHERE visibility = 'Public';
                ",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_note_text_search;")
            .await?;

        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_note_cw_search;")
            .await?;

        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_user_search;")
            .await?;

        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_note_trending;")
            .await?;

        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_user_popular;")
            .await?;

        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_note_timeline_public;")
            .await?;

        Ok(())
    }
}
