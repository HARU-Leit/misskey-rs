//! Instance repository for federation management.

use std::sync::Arc;

use crate::entities::{instance, Instance};
use misskey_common::{AppError, AppResult, IdGenerator};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

/// Instance repository for database operations.
#[derive(Clone)]
pub struct InstanceRepository {
    db: Arc<DatabaseConnection>,
    id_gen: IdGenerator,
}

impl InstanceRepository {
    /// Create a new instance repository.
    #[must_use]
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            id_gen: IdGenerator::new(),
        }
    }

    /// Find an instance by ID.
    pub async fn find_by_id(&self, id: &str) -> AppResult<Option<instance::Model>> {
        Instance::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find an instance by hostname.
    pub async fn find_by_host(&self, host: &str) -> AppResult<Option<instance::Model>> {
        Instance::find()
            .filter(instance::Column::Host.eq(host.to_lowercase()))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get an instance by hostname, or error if not found.
    pub async fn get_by_host(&self, host: &str) -> AppResult<instance::Model> {
        self.find_by_host(host)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Instance not found: {host}")))
    }

    /// Create a new instance.
    pub async fn create(&self, model: instance::ActiveModel) -> AppResult<instance::Model> {
        model
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Update an instance.
    pub async fn update(&self, model: instance::ActiveModel) -> AppResult<instance::Model> {
        model
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Find or create an instance by hostname.
    pub async fn find_or_create(&self, host: &str) -> AppResult<instance::Model> {
        let host_lower = host.to_lowercase();
        if let Some(instance) = self.find_by_host(&host_lower).await? {
            return Ok(instance);
        }

        // Create new instance
        let id = self.id_gen.generate();
        let now = chrono::Utc::now().fixed_offset();

        let model = instance::ActiveModel {
            id: Set(id),
            host: Set(host_lower),
            created_at: Set(now),
            ..Default::default()
        };

        self.create(model).await
    }

    /// List all blocked instances.
    pub async fn find_blocked(&self, limit: u64, offset: u64) -> AppResult<Vec<instance::Model>> {
        Instance::find()
            .filter(instance::Column::IsBlocked.eq(true))
            .order_by_asc(instance::Column::Host)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// List all silenced instances.
    pub async fn find_silenced(&self, limit: u64, offset: u64) -> AppResult<Vec<instance::Model>> {
        Instance::find()
            .filter(instance::Column::IsSilenced.eq(true))
            .order_by_asc(instance::Column::Host)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// List all suspended instances.
    pub async fn find_suspended(&self, limit: u64, offset: u64) -> AppResult<Vec<instance::Model>> {
        Instance::find()
            .filter(instance::Column::IsSuspended.eq(true))
            .order_by_asc(instance::Column::Host)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// List all instances (paginated).
    pub async fn find_all(
        &self,
        limit: u64,
        offset: u64,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
    ) -> AppResult<Vec<instance::Model>> {
        let mut query = Instance::find();

        let column = match sort_by {
            Some("host") => instance::Column::Host,
            Some("usersCount") => instance::Column::UsersCount,
            Some("notesCount") => instance::Column::NotesCount,
            Some("followingCount") => instance::Column::FollowingCount,
            Some("followersCount") => instance::Column::FollowersCount,
            Some("lastCommunicatedAt") => instance::Column::LastCommunicatedAt,
            _ => instance::Column::CreatedAt,
        };

        query = match sort_order {
            Some("asc") | Some("ASC") => query.order_by_asc(column),
            _ => query.order_by_desc(column),
        };

        query
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Search instances by host.
    pub async fn search(
        &self,
        query: &str,
        limit: u64,
        offset: u64,
    ) -> AppResult<Vec<instance::Model>> {
        Instance::find()
            .filter(instance::Column::Host.contains(query.to_lowercase()))
            .order_by_asc(instance::Column::Host)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Check if an instance is blocked.
    pub async fn is_blocked(&self, host: &str) -> AppResult<bool> {
        let instance = self.find_by_host(host).await?;
        Ok(instance.is_some_and(|i| i.is_blocked))
    }

    /// Check if an instance is silenced.
    pub async fn is_silenced(&self, host: &str) -> AppResult<bool> {
        let instance = self.find_by_host(host).await?;
        Ok(instance.is_some_and(|i| i.is_silenced))
    }

    /// Block an instance.
    pub async fn block(&self, host: &str) -> AppResult<instance::Model> {
        let instance = self.find_or_create(host).await?;
        let now = chrono::Utc::now().fixed_offset();

        let model = instance::ActiveModel {
            id: Set(instance.id),
            is_blocked: Set(true),
            updated_at: Set(Some(now)),
            ..Default::default()
        };

        self.update(model).await
    }

    /// Unblock an instance.
    pub async fn unblock(&self, host: &str) -> AppResult<instance::Model> {
        let instance = self.get_by_host(host).await?;
        let now = chrono::Utc::now().fixed_offset();

        let model = instance::ActiveModel {
            id: Set(instance.id),
            is_blocked: Set(false),
            updated_at: Set(Some(now)),
            ..Default::default()
        };

        self.update(model).await
    }

    /// Silence an instance.
    pub async fn silence(&self, host: &str) -> AppResult<instance::Model> {
        let instance = self.find_or_create(host).await?;
        let now = chrono::Utc::now().fixed_offset();

        let model = instance::ActiveModel {
            id: Set(instance.id),
            is_silenced: Set(true),
            updated_at: Set(Some(now)),
            ..Default::default()
        };

        self.update(model).await
    }

    /// Unsilence an instance.
    pub async fn unsilence(&self, host: &str) -> AppResult<instance::Model> {
        let instance = self.get_by_host(host).await?;
        let now = chrono::Utc::now().fixed_offset();

        let model = instance::ActiveModel {
            id: Set(instance.id),
            is_silenced: Set(false),
            updated_at: Set(Some(now)),
            ..Default::default()
        };

        self.update(model).await
    }

    /// Suspend an instance.
    pub async fn suspend(&self, host: &str) -> AppResult<instance::Model> {
        let instance = self.find_or_create(host).await?;
        let now = chrono::Utc::now().fixed_offset();

        let model = instance::ActiveModel {
            id: Set(instance.id),
            is_suspended: Set(true),
            updated_at: Set(Some(now)),
            ..Default::default()
        };

        self.update(model).await
    }

    /// Unsuspend an instance.
    pub async fn unsuspend(&self, host: &str) -> AppResult<instance::Model> {
        let instance = self.get_by_host(host).await?;
        let now = chrono::Utc::now().fixed_offset();

        let model = instance::ActiveModel {
            id: Set(instance.id),
            is_suspended: Set(false),
            updated_at: Set(Some(now)),
            ..Default::default()
        };

        self.update(model).await
    }

    /// Update moderation note for an instance.
    pub async fn update_moderation_note(
        &self,
        host: &str,
        note: Option<String>,
    ) -> AppResult<instance::Model> {
        let instance = self.get_by_host(host).await?;
        let now = chrono::Utc::now().fixed_offset();

        let model = instance::ActiveModel {
            id: Set(instance.id),
            moderation_note: Set(note),
            updated_at: Set(Some(now)),
            ..Default::default()
        };

        self.update(model).await
    }

    /// Update instance info (from nodeinfo).
    pub async fn update_info(
        &self,
        host: &str,
        software_name: Option<String>,
        software_version: Option<String>,
        name: Option<String>,
        description: Option<String>,
        maintainer_email: Option<String>,
        maintainer_name: Option<String>,
        icon_url: Option<String>,
        theme_color: Option<String>,
    ) -> AppResult<instance::Model> {
        let instance = self.find_or_create(host).await?;
        let now = chrono::Utc::now().fixed_offset();

        let model = instance::ActiveModel {
            id: Set(instance.id),
            software_name: Set(software_name),
            software_version: Set(software_version),
            name: Set(name),
            description: Set(description),
            maintainer_email: Set(maintainer_email),
            maintainer_name: Set(maintainer_name),
            icon_url: Set(icon_url),
            theme_color: Set(theme_color),
            info_updated_at: Set(Some(now)),
            is_nodeinfo_fetched: Set(true),
            updated_at: Set(Some(now)),
            ..Default::default()
        };

        self.update(model).await
    }

    /// Update last communicated timestamp.
    pub async fn touch_last_communicated(&self, host: &str) -> AppResult<()> {
        let instance = self.find_or_create(host).await?;
        let now = chrono::Utc::now().fixed_offset();

        let model = instance::ActiveModel {
            id: Set(instance.id),
            last_communicated_at: Set(Some(now)),
            ..Default::default()
        };

        self.update(model).await?;
        Ok(())
    }

    /// Increment user count for an instance.
    pub async fn increment_users_count(&self, host: &str) -> AppResult<()> {
        let instance = self.find_or_create(host).await?;

        let model = instance::ActiveModel {
            id: Set(instance.id),
            users_count: Set(instance.users_count + 1),
            ..Default::default()
        };

        self.update(model).await?;
        Ok(())
    }

    /// Increment note count for an instance.
    pub async fn increment_notes_count(&self, host: &str) -> AppResult<()> {
        let instance = self.find_or_create(host).await?;

        let model = instance::ActiveModel {
            id: Set(instance.id),
            notes_count: Set(instance.notes_count + 1),
            ..Default::default()
        };

        self.update(model).await?;
        Ok(())
    }

    /// Get statistics for federation dashboard.
    pub async fn get_stats(&self) -> AppResult<InstanceStats> {
        let total = Instance::find()
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let blocked = Instance::find()
            .filter(instance::Column::IsBlocked.eq(true))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let silenced = Instance::find()
            .filter(instance::Column::IsSilenced.eq(true))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let suspended = Instance::find()
            .filter(instance::Column::IsSuspended.eq(true))
            .count(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(InstanceStats {
            total,
            blocked,
            silenced,
            suspended,
        })
    }
}

/// Statistics about instances.
#[derive(Debug, Clone)]
pub struct InstanceStats {
    /// Total number of instances.
    pub total: u64,
    /// Number of blocked instances.
    pub blocked: u64,
    /// Number of silenced instances.
    pub silenced: u64,
    /// Number of suspended instances.
    pub suspended: u64,
}
