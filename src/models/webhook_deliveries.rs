//! Persisted webhook deliveries: one row per event/webhook pair.

pub use super::_entities::webhook_deliveries::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type WebhookDeliveries = Entity;

pub const STATUS_PENDING: &str = "pending";
pub const STATUS_SUCCESS: &str = "success";
pub const STATUS_FAILED: &str = "failed";

impl Model {
    /// Recent deliveries for a webhook, newest first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_webhook(
        db: &DatabaseConnection,
        webhook_id: i64,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        Entity::find()
            .filter(Column::WebhookId.eq(webhook_id))
            .order_by_desc(Column::Id)
            .limit(limit)
            .all(db)
            .await
            .map_err(Into::into)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Records a pending delivery.
    ///
    /// # Errors
    /// Returns a database error if the insert fails.
    pub async fn create(
        db: &DatabaseConnection,
        webhook_id: i64,
        event: &str,
        payload: &serde_json::Value,
    ) -> Result<Model> {
        ActiveModel {
            webhook_id: Set(webhook_id),
            event: Set(event.to_owned()),
            payload: Set(payload.to_string()),
            status: Set(STATUS_PENDING.to_owned()),
            attempts: Set(0),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(Into::into)
    }

    /// Marks a delivery successful after a 2xx response.
    ///
    /// # Errors
    /// Returns a database error if the update fails.
    pub async fn mark_success(db: &DatabaseConnection, delivery: Model) -> Result<Model> {
        let attempts = delivery.attempts + 1;
        let mut active: ActiveModel = delivery.into();
        active.status = Set(STATUS_SUCCESS.to_owned());
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
        delivery: Model,
        error: &str,
    ) -> Result<Model> {
        let attempts = delivery.attempts + 1;
        let mut active: ActiveModel = delivery.into();
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
