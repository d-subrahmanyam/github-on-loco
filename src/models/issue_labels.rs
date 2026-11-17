//! Join table between issues and labels.

pub use super::_entities::issue_labels::Column;
pub use super::_entities::issue_labels::{ActiveModel, Entity, Model};
use super::labels;
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;
use sea_orm::TransactionTrait;

pub type IssueLabels = Entity;

/// Custom finders and cache-style replacement.
impl Entity {
    /// The labels attached to an issue.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn labels_for_issue(
        db: &DatabaseConnection,
        issue_id: i64,
    ) -> ModelResult<Vec<labels::Model>> {
        let joins = Entity::find()
            .filter(Column::IssueId.eq(issue_id))
            .all(db)
            .await?;
        let ids: Vec<i64> = joins.into_iter().map(|join| join.label_id).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        labels::Entity::find()
            .filter(labels::Column::Id.is_in(ids))
            .order_by_asc(labels::Column::Name)
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// Replaces an issue's labels with the given set, atomically.
    ///
    /// # Errors
    /// Returns a database error if any statement fails.
    pub async fn replace_for_issue(
        db: &DatabaseConnection,
        issue_id: i64,
        label_ids: &[i64],
    ) -> Result<()> {
        let txn = db.begin().await?;
        Entity::delete_many()
            .filter(Column::IssueId.eq(issue_id))
            .exec(&txn)
            .await?;
        for label_id in label_ids {
            ActiveModel {
                issue_id: Set(issue_id),
                label_id: Set(*label_id),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
        txn.commit().await?;
        Ok(())
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
