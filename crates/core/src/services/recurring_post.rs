//! Recurring post service for automatic repeated posting.

use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use misskey_common::{AppError, AppResult};
use misskey_db::entities::recurring_post::{self, RecurringInterval, RecurringVisibility};
use misskey_db::repositories::{
    CreateRecurringPostInput, RecurringPostRepository, UpdateRecurringPostInput,
};
use serde::Deserialize;
use validator::Validate;

/// Maximum number of active recurring posts per user.
const MAX_RECURRING_POSTS_PER_USER: u64 = 20;

/// Input for creating a recurring post.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateRecurringInput {
    #[validate(length(max = 3000))]
    pub text: Option<String>,
    #[validate(length(max = 512))]
    pub cw: Option<String>,
    #[serde(default = "default_visibility")]
    pub visibility: RecurringVisibility,
    #[serde(default)]
    pub local_only: bool,
    #[serde(default)]
    #[validate(length(max = 16))]
    pub file_ids: Vec<String>,
    pub interval: RecurringInterval,
    pub day_of_week: Option<i16>,
    pub day_of_month: Option<i16>,
    #[validate(range(min = 0, max = 23))]
    pub hour: i16,
    #[validate(range(min = 0, max = 59))]
    pub minute: i16,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    pub max_posts: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
}

const fn default_visibility() -> RecurringVisibility {
    RecurringVisibility::Public
}

fn default_timezone() -> String {
    "UTC".to_string()
}

/// Input for updating a recurring post.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecurringInput {
    pub id: String,
    #[validate(length(max = 3000))]
    pub text: Option<Option<String>>,
    #[validate(length(max = 512))]
    pub cw: Option<Option<String>>,
    pub visibility: Option<RecurringVisibility>,
    pub local_only: Option<bool>,
    #[validate(length(max = 16))]
    pub file_ids: Option<Vec<String>>,
    pub interval: Option<RecurringInterval>,
    pub day_of_week: Option<Option<i16>>,
    pub day_of_month: Option<Option<i16>>,
    #[validate(range(min = 0, max = 23))]
    pub hour: Option<i16>,
    #[validate(range(min = 0, max = 59))]
    pub minute: Option<i16>,
    pub timezone: Option<String>,
    pub is_active: Option<bool>,
    pub max_posts: Option<Option<i32>>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
}

/// Service for managing recurring posts.
#[derive(Clone)]
pub struct RecurringPostService {
    recurring_post_repo: RecurringPostRepository,
}

impl RecurringPostService {
    /// Create a new recurring post service.
    #[must_use]
    pub const fn new(recurring_post_repo: RecurringPostRepository) -> Self {
        Self {
            recurring_post_repo,
        }
    }

    /// Get a recurring post by ID.
    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<recurring_post::Model>> {
        self.recurring_post_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Get a recurring post by ID with ownership check.
    pub async fn get_by_id_for_user(
        &self,
        id: &str,
        user_id: &str,
    ) -> AppResult<recurring_post::Model> {
        let post = self
            .recurring_post_repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Recurring post not found".to_string()))?;

        if post.user_id != user_id {
            return Err(AppError::Forbidden(
                "Not the owner of this recurring post".to_string(),
            ));
        }

        Ok(post)
    }

    /// List recurring posts for a user.
    pub async fn list_posts(&self, user_id: &str) -> AppResult<Vec<recurring_post::Model>> {
        self.recurring_post_repo
            .find_by_user(user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// List active recurring posts for a user.
    pub async fn list_active_posts(&self, user_id: &str) -> AppResult<Vec<recurring_post::Model>> {
        self.recurring_post_repo
            .find_active_by_user(user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count recurring posts for a user.
    pub async fn count_posts(&self, user_id: &str) -> AppResult<u64> {
        self.recurring_post_repo
            .count_by_user(user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Count active recurring posts for a user.
    pub async fn count_active_posts(&self, user_id: &str) -> AppResult<u64> {
        self.recurring_post_repo
            .count_active_by_user(user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Create a new recurring post.
    pub async fn create(
        &self,
        user_id: &str,
        input: CreateRecurringInput,
    ) -> AppResult<recurring_post::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Check content exists
        if input.text.is_none() && input.file_ids.is_empty() {
            return Err(AppError::Validation(
                "Recurring post must have text or files".to_string(),
            ));
        }

        // Check active post limit
        let active_count = self
            .recurring_post_repo
            .count_active_by_user(user_id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        if active_count >= MAX_RECURRING_POSTS_PER_USER {
            return Err(AppError::Validation(format!(
                "Maximum of {MAX_RECURRING_POSTS_PER_USER} active recurring posts allowed"
            )));
        }

        // Validate timezone
        let tz: Tz = input
            .timezone
            .parse()
            .map_err(|_| AppError::Validation("Invalid timezone".to_string()))?;

        // Validate interval-specific fields
        match input.interval {
            RecurringInterval::Weekly => {
                if input.day_of_week.is_none() {
                    return Err(AppError::Validation(
                        "Weekly recurring posts require day_of_week".to_string(),
                    ));
                }
                let dow = input.day_of_week.unwrap();
                if !(0..=6).contains(&dow) {
                    return Err(AppError::Validation(
                        "day_of_week must be 0-6 (Sunday-Saturday)".to_string(),
                    ));
                }
            }
            RecurringInterval::Monthly => {
                if input.day_of_month.is_none() {
                    return Err(AppError::Validation(
                        "Monthly recurring posts require day_of_month".to_string(),
                    ));
                }
                let dom = input.day_of_month.unwrap();
                if !(1..=31).contains(&dom) {
                    return Err(AppError::Validation(
                        "day_of_month must be 1-31".to_string(),
                    ));
                }
            }
            RecurringInterval::Daily => {}
        }

        // Convert local time to UTC
        let (hour_utc, minute_utc) = convert_to_utc(input.hour, input.minute, &tz)?;

        // Create the recurring post
        let db_input = CreateRecurringPostInput {
            user_id: user_id.to_string(),
            text: input.text,
            cw: input.cw,
            visibility: input.visibility,
            local_only: input.local_only,
            file_ids: input.file_ids,
            interval: input.interval,
            day_of_week: input.day_of_week,
            day_of_month: input.day_of_month,
            hour_utc,
            minute_utc,
            timezone: input.timezone,
            max_posts: input.max_posts,
            expires_at: input.expires_at,
        };

        let mut post = self
            .recurring_post_repo
            .create(db_input)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Calculate and set next post time
        let next_post_at = self.calculate_next_post_time(&post)?;
        if let Some(updated) = self
            .recurring_post_repo
            .update_next_post_at(&post.id, Some(next_post_at))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
        {
            post = updated;
        }

        Ok(post)
    }

    /// Update a recurring post.
    pub async fn update(
        &self,
        user_id: &str,
        input: UpdateRecurringInput,
    ) -> AppResult<recurring_post::Model> {
        // Validate input
        input
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // Get post and verify ownership
        let post = self.get_by_id_for_user(&input.id, user_id).await?;

        // Validate timezone if provided
        let tz: Option<Tz> = if let Some(ref timezone) = input.timezone {
            Some(
                timezone
                    .parse()
                    .map_err(|_| AppError::Validation("Invalid timezone".to_string()))?,
            )
        } else {
            None
        };

        // Calculate UTC time if local time changed
        let (hour_utc, minute_utc) = if input.hour.is_some() || input.minute.is_some() {
            let timezone = input.timezone.as_ref().unwrap_or(&post.timezone);
            let tz: Tz = timezone
                .parse()
                .map_err(|_| AppError::Validation("Invalid timezone".to_string()))?;
            let hour = input.hour.unwrap_or(post.hour_utc);
            let minute = input.minute.unwrap_or(post.minute_utc);
            let (h, m) = convert_to_utc(hour, minute, &tz)?;
            (Some(h), Some(m))
        } else if tz.is_some() {
            // Timezone changed but time didn't, recalculate UTC
            let tz = tz.unwrap();
            let (h, m) = convert_to_utc(post.hour_utc, post.minute_utc, &tz)?;
            (Some(h), Some(m))
        } else {
            (None, None)
        };

        // Check if schedule changed before moving values
        let schedule_changed =
            hour_utc.is_some() || input.interval.is_some() || input.is_active == Some(true);

        let db_input = UpdateRecurringPostInput {
            text: input.text,
            cw: input.cw,
            visibility: input.visibility,
            local_only: input.local_only,
            file_ids: input.file_ids,
            interval: input.interval,
            day_of_week: input.day_of_week,
            day_of_month: input.day_of_month,
            hour_utc,
            minute_utc,
            timezone: input.timezone,
            is_active: input.is_active,
            max_posts: input.max_posts,
            expires_at: input.expires_at,
        };

        let updated = self
            .recurring_post_repo
            .update(&input.id, db_input)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Recurring post not found".to_string()))?;

        // Recalculate next post time if schedule changed
        if schedule_changed {
            let next_post_at = self.calculate_next_post_time(&updated)?;
            if let Some(final_post) = self
                .recurring_post_repo
                .update_next_post_at(&updated.id, Some(next_post_at))
                .await
                .map_err(|e| AppError::Database(e.to_string()))?
            {
                return Ok(final_post);
            }
        }

        Ok(updated)
    }

    /// Activate a recurring post.
    pub async fn activate(&self, id: &str, user_id: &str) -> AppResult<recurring_post::Model> {
        // Verify ownership
        self.get_by_id_for_user(id, user_id).await?;

        let post = self
            .recurring_post_repo
            .activate(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Recurring post not found".to_string()))?;

        // Calculate next post time
        let next_post_at = self.calculate_next_post_time(&post)?;
        self.recurring_post_repo
            .update_next_post_at(id, Some(next_post_at))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Recurring post not found".to_string()))
    }

    /// Deactivate a recurring post.
    pub async fn deactivate(&self, id: &str, user_id: &str) -> AppResult<recurring_post::Model> {
        // Verify ownership
        self.get_by_id_for_user(id, user_id).await?;

        self.recurring_post_repo
            .deactivate(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Recurring post not found".to_string()))
    }

    /// Delete a recurring post.
    pub async fn delete(&self, id: &str, user_id: &str) -> AppResult<()> {
        // Verify ownership
        self.get_by_id_for_user(id, user_id).await?;

        self.recurring_post_repo
            .delete(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    // ==================== Processing Methods (for job queue) ====================

    /// Find recurring posts that are due for execution.
    pub async fn find_due_posts(&self) -> AppResult<Vec<recurring_post::Model>> {
        let now = Utc::now();
        self.recurring_post_repo
            .find_due_posts(now)
            .await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    /// Record that a post was executed and calculate next post time.
    pub async fn record_execution(&self, id: &str) -> AppResult<Option<recurring_post::Model>> {
        let Some(post) = self
            .recurring_post_repo
            .record_post_execution(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
        else {
            return Ok(None);
        };

        // If still active, calculate next post time
        if post.is_active {
            let next_post_at = self.calculate_next_post_time(&post)?;
            return self
                .recurring_post_repo
                .update_next_post_at(id, Some(next_post_at))
                .await
                .map_err(|e| AppError::Database(e.to_string()));
        }

        Ok(Some(post))
    }

    /// Calculate the next post time for a recurring post.
    fn calculate_next_post_time(&self, post: &recurring_post::Model) -> AppResult<DateTime<Utc>> {
        let now = Utc::now();
        let tz: Tz = post
            .timezone
            .parse()
            .map_err(|_| AppError::Internal("Invalid timezone stored".to_string()))?;

        // Convert current time to user's timezone
        let now_local = now.with_timezone(&tz);

        // Create target time today in user's timezone
        let today_target = tz
            .with_ymd_and_hms(
                now_local.year(),
                now_local.month(),
                now_local.day(),
                post.hour_utc as u32,
                post.minute_utc as u32,
                0,
            )
            .single()
            .ok_or_else(|| AppError::Internal("Failed to calculate next post time".to_string()))?;

        let next = match post.interval {
            RecurringInterval::Daily => {
                if today_target > now_local {
                    today_target
                } else {
                    today_target + Duration::days(1)
                }
            }
            RecurringInterval::Weekly => {
                let target_weekday = match post.day_of_week.unwrap_or(0) {
                    0 => Weekday::Sun,
                    1 => Weekday::Mon,
                    2 => Weekday::Tue,
                    3 => Weekday::Wed,
                    4 => Weekday::Thu,
                    5 => Weekday::Fri,
                    _ => Weekday::Sat,
                };

                let current_weekday = now_local.weekday();
                let days_until = (target_weekday.num_days_from_sunday() as i64
                    - current_weekday.num_days_from_sunday() as i64
                    + 7)
                    % 7;

                let target = today_target + Duration::days(days_until);

                // If target is today but time has passed, move to next week
                if days_until == 0 && target <= now_local {
                    target + Duration::days(7)
                } else {
                    target
                }
            }
            RecurringInterval::Monthly => {
                let target_day = post.day_of_month.unwrap_or(1) as u32;

                // Try this month first
                let this_month = tz
                    .with_ymd_and_hms(
                        now_local.year(),
                        now_local.month(),
                        target_day.min(days_in_month(now_local.year(), now_local.month())),
                        post.hour_utc as u32,
                        post.minute_utc as u32,
                        0,
                    )
                    .single();

                if let Some(target) = this_month {
                    if target > now_local {
                        target
                    } else {
                        // Move to next month
                        let (next_year, next_month) = if now_local.month() == 12 {
                            (now_local.year() + 1, 1)
                        } else {
                            (now_local.year(), now_local.month() + 1)
                        };

                        tz.with_ymd_and_hms(
                            next_year,
                            next_month,
                            target_day.min(days_in_month(next_year, next_month)),
                            post.hour_utc as u32,
                            post.minute_utc as u32,
                            0,
                        )
                        .single()
                        .ok_or_else(|| {
                            AppError::Internal("Failed to calculate next post time".to_string())
                        })?
                    }
                } else {
                    // Invalid day for this month, try next month
                    let (next_year, next_month) = if now_local.month() == 12 {
                        (now_local.year() + 1, 1)
                    } else {
                        (now_local.year(), now_local.month() + 1)
                    };

                    tz.with_ymd_and_hms(
                        next_year,
                        next_month,
                        target_day.min(days_in_month(next_year, next_month)),
                        post.hour_utc as u32,
                        post.minute_utc as u32,
                        0,
                    )
                    .single()
                    .ok_or_else(|| {
                        AppError::Internal("Failed to calculate next post time".to_string())
                    })?
                }
            }
        };

        Ok(next.with_timezone(&Utc))
    }
}

/// Convert local time to UTC based on timezone.
fn convert_to_utc(hour: i16, minute: i16, tz: &Tz) -> AppResult<(i16, i16)> {
    let now = Utc::now();
    let today = now.with_timezone(tz);

    // Create a datetime in the target timezone
    let local_time = tz
        .with_ymd_and_hms(
            today.year(),
            today.month(),
            today.day(),
            hour as u32,
            minute as u32,
            0,
        )
        .single()
        .ok_or_else(|| AppError::Validation("Invalid time".to_string()))?;

    // Convert to UTC
    let utc_time = local_time.with_timezone(&Utc);

    Ok((utc_time.hour() as i16, utc_time.minute() as i16))
}

/// Get the number of days in a month.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2024, 1), 31);
        assert_eq!(days_in_month(2024, 2), 29); // Leap year
        assert_eq!(days_in_month(2023, 2), 28); // Not leap year
        assert_eq!(days_in_month(2024, 4), 30);
    }

    #[test]
    fn test_convert_to_utc() {
        let tz: Tz = "Asia/Tokyo".parse().unwrap();
        let (hour, minute) = convert_to_utc(12, 0, &tz).unwrap();
        // Tokyo is UTC+9, so 12:00 JST = 03:00 UTC
        assert_eq!(hour, 3);
        assert_eq!(minute, 0);
    }
}
