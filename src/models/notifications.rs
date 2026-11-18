//! Pending and delivered notifications, one row per recipient and channel.

pub use super::_entities::notifications::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type Notifications = Entity;

pub const STATUS_PENDING: &str = "pending";
pub const STATUS_SENT: &str = "sent";
pub const STATUS_FAILED: &str = "failed";

impl Model {
    /// A user's notifications, newest first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_user(
        db: &DatabaseConnection,
        user_id: i64,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .order_by_desc(Column::Id)
            .limit(limit)
            .all(db)
            .await
            .map_err(Into::into)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Records a pending notification.
    ///
    /// # Errors
    /// Returns a database error if the insert fails.
    pub async fn create(
        db: &DatabaseConnection,
        user_id: i64,
        repository_id: i64,
        event: &str,
        title: &str,
        body: &str,
        channel: &str,
    ) -> Result<Model> {
        ActiveModel {
            user_id: Set(user_id),
            repository_id: Set(repository_id),
            event: Set(event.to_owned()),
            title: Set(title.to_owned()),
            body: Set(body.to_owned()),
            channel: Set(channel.to_owned()),
            status: Set(STATUS_PENDING.to_owned()),
            attempts: Set(0),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(Into::into)
    }

    /// Marks a notification delivered.
    ///
    /// # Errors
    /// Returns a database error if the update fails.
    pub async fn mark_sent(db: &DatabaseConnection, notification: Model) -> Result<Model> {
        let attempts = notification.attempts + 1;
        let mut active: ActiveModel = notification.into();
        active.status = Set(STATUS_SENT.to_owned());
        active.attempts = Set(attempts);
        active.last_error = Set(None);
        active.delivered_at = Set(Some(chrono::Utc::now().into()));
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(db).await.map_err(Into::into)
    }

    /// Records a failed attempt; the queue decides whether to retry.
    ///
    /// # Errors
    /// Returns a database error if the update fails.
    pub async fn mark_failed(
        db: &DatabaseConnection,
        notification: Model,
        error: &str,
    ) -> Result<Model> {
        let attempts = notification.attempts + 1;
        let mut active: ActiveModel = notification.into();
        active.status = Set(STATUS_FAILED.to_owned());
        active.attempts = Set(attempts);
        active.last_error = Set(Some(error.chars().take(500).collect()));
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(db).await.map_err(Into::into)
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}
