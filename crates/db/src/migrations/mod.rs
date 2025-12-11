//! Database migrations.
//!
//! Schema migrations for the database.

#![allow(missing_docs)]

use sea_orm_migration::prelude::*;

mod m20250101_000001_create_user_table;
mod m20250101_000002_create_note_table;
mod m20250101_000003_create_user_profile_table;
mod m20250101_000004_create_following_table;
mod m20250101_000005_create_follow_request_table;
mod m20250101_000006_create_blocking_table;
mod m20250101_000007_create_muting_table;
mod m20250101_000008_create_drive_file_table;
mod m20250101_000009_create_reaction_table;
mod m20250101_000010_create_notification_table;
mod m20250101_000011_create_user_keypair_table;
mod m20250101_000012_create_drive_folder_table;
mod m20250101_000013_create_emoji_table;
mod m20250101_000014_create_announcement_table;
mod m20250101_000015_create_messaging_message_table;
mod m20250101_000016_add_fulltext_search;
mod m20250101_000017_create_clip_table;
mod m20250101_000018_add_pinned_note_ids;
mod m20250101_000019_create_note_edit_table;
mod m20250101_000020_create_word_filter_table;
mod m20250101_000021_create_scheduled_note_table;
mod m20250101_000022_add_two_factor_backup_codes;
mod m20250101_000023_create_antenna_table;
mod m20250101_000024_create_channel_table;
mod m20250101_000025_create_instance_table;
mod m20250101_000026_create_security_key_table;
mod m20250101_000027_create_oauth_tables;
mod m20250101_000028_create_webhook_table;
mod m20250101_000029_create_page_table;
mod m20250101_000030_create_gallery_table;
mod m20250101_000031_create_push_subscription_table;
mod m20250101_000032_add_account_migration_fields;
mod m20250101_000033_create_group_table;
mod m20250101_000034_add_channel_id_to_note;
mod m20250101_000035_add_community_features;
mod m20250101_000036_add_admin_settings;
mod m20250101_000037_add_bubble_instances;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250101_000001_create_user_table::Migration),
            Box::new(m20250101_000002_create_note_table::Migration),
            Box::new(m20250101_000003_create_user_profile_table::Migration),
            Box::new(m20250101_000004_create_following_table::Migration),
            Box::new(m20250101_000005_create_follow_request_table::Migration),
            Box::new(m20250101_000006_create_blocking_table::Migration),
            Box::new(m20250101_000007_create_muting_table::Migration),
            Box::new(m20250101_000008_create_drive_file_table::Migration),
            Box::new(m20250101_000009_create_reaction_table::Migration),
            Box::new(m20250101_000010_create_notification_table::Migration),
            Box::new(m20250101_000011_create_user_keypair_table::Migration),
            Box::new(m20250101_000012_create_drive_folder_table::Migration),
            Box::new(m20250101_000013_create_emoji_table::Migration),
            Box::new(m20250101_000014_create_announcement_table::Migration),
            Box::new(m20250101_000015_create_messaging_message_table::Migration),
            Box::new(m20250101_000016_add_fulltext_search::Migration),
            Box::new(m20250101_000017_create_clip_table::Migration),
            Box::new(m20250101_000018_add_pinned_note_ids::Migration),
            Box::new(m20250101_000019_create_note_edit_table::Migration),
            Box::new(m20250101_000020_create_word_filter_table::Migration),
            Box::new(m20250101_000021_create_scheduled_note_table::Migration),
            Box::new(m20250101_000022_add_two_factor_backup_codes::Migration),
            Box::new(m20250101_000023_create_antenna_table::Migration),
            Box::new(m20250101_000024_create_channel_table::Migration),
            Box::new(m20250101_000025_create_instance_table::Migration),
            Box::new(m20250101_000026_create_security_key_table::Migration),
            Box::new(m20250101_000027_create_oauth_tables::Migration),
            Box::new(m20250101_000028_create_webhook_table::Migration),
            Box::new(m20250101_000029_create_page_table::Migration),
            Box::new(m20250101_000030_create_gallery_table::Migration),
            Box::new(m20250101_000031_create_push_subscription_table::Migration),
            Box::new(m20250101_000032_add_account_migration_fields::Migration),
            Box::new(m20250101_000033_create_group_table::Migration),
            Box::new(m20250101_000034_add_channel_id_to_note::Migration),
            Box::new(m20250101_000035_add_community_features::Migration),
            Box::new(m20250101_000036_add_admin_settings::Migration),
            Box::new(m20250101_000037_add_bubble_instances::Migration),
        ]
    }
}
