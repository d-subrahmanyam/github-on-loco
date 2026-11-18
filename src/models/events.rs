//! Activity events for a repository. Events are an append-only log used by the
//! SPA activity feed; git remains the source of truth for code.

pub use super::_entities::events::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;
use serde_json::Value;

pub type Events = Entity;

/// Event kinds recorded by the app.
pub mod kind {
    pub const STAR: &str = "star";
    pub const UNSTAR: &str = "unstar";
    pub const WATCH: &str = "watch";
    pub const UNWATCH: &str = "unwatch";
    pub const ISSUE_OPENED: &str = "issue.opened";
    pub const ISSUE_CLOSED: &str = "issue.closed";
    pub const ISSUE_REOPENED: &str = "issue.reopened";
    pub const PR_OPENED: &str = "pr.opened";
    pub const PR_MERGED: &str = "pr.merged";
    pub const PR_APPROVED: &str = "pr.approved";
    pub const PR_CHANGES_REQUESTED: &str = "pr.changes_requested";
    pub const PR_REVIEWED: &str = "pr.reviewed";
    pub const COLLABORATOR_ADDED: &str = "collaborator.added";
    pub const COLLABORATOR_REMOVED: &str = "collaborator.removed";
    pub const PUSH: &str = "push";
}

impl Entity {
    /// Appends an event. The payload holds small display hints (title, branch),
    /// never authoritative state.
    ///
    /// # Errors
    /// Returns a database error if the insert fails.
    pub async fn record(
        db: &DatabaseConnection,
        repository_id: i64,
        actor_id: i64,
        kind: &str,
        payload: Option<Value>,
    ) -> Result<Model> {
        ActiveModel {
            repository_id: Set(repository_id),
            user_id: Set(actor_id),
            kind: Set(kind.to_owned()),
            payload: Set(payload),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(Into::into)
    }

    /// Most recent events first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_repository(
        db: &DatabaseConnection,
        repository_id: i64,
        limit: usize,
    ) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .order_by_desc(Column::Id)
            .limit(limit.clamp(1, 100) as u64)
            .all(db)
            .await
            .map_err(Into::into)
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
