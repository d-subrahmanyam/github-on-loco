//! Issue domain model: per-repository numbering, state and labels.

pub use super::_entities::issues::Column;
pub use super::_entities::issues::{ActiveModel, Entity, Model};
use super::{issue_comments, issue_labels, labels};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;
use sea_orm::{PaginatorTrait, TransactionTrait};

pub type Issues = Entity;

/// State values stored in the `state` column.
pub const STATE_OPEN: &str = "open";
pub const STATE_CLOSED: &str = "closed";

/// Read-oriented domain logic.
impl Model {
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.state == STATE_OPEN
    }

    /// Issues in a repository, optionally filtered by state.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_repository(
        db: &DatabaseConnection,
        repository_id: i64,
        state: Option<&str>,
    ) -> ModelResult<Vec<Self>> {
        let mut query = Entity::find().filter(Column::RepositoryId.eq(repository_id));
        if let Some(state) = state {
            if state == STATE_OPEN || state == STATE_CLOSED {
                query = query.filter(Column::State.eq(state));
            }
        }
        query
            .order_by_desc(Column::Number)
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// Look up an issue by its per-repository number.
    ///
    /// # Errors
    /// Returns `EntityNotFound` when the repository has no such issue.
    pub async fn find_by_number(
        db: &DatabaseConnection,
        repository_id: i64,
        number: i64,
    ) -> ModelResult<Self> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .filter(Column::Number.eq(number))
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Number of comments on this issue.
    ///
    /// # Errors
    /// Returns a database error if the count fails.
    pub async fn comment_count(&self, db: &DatabaseConnection) -> Result<i64> {
        Ok(issue_comments::Entity::find()
            .filter(issue_comments::Column::IssueId.eq(self.id))
            .count(db)
            .await? as i64)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Creates an issue, allocates its number and attaches labels atomically.
    ///
    /// The highest existing number is read inside the same transaction as the
    /// insert, so two concurrent creations cannot claim the same number.
    ///
    /// # Errors
    /// Returns a 400 for a label from another repository, or a database error.
    pub async fn create_for_repository(
        db: &DatabaseConnection,
        repository_id: i64,
        user_id: i64,
        title: &str,
        body: Option<&str>,
        label_ids: &[i64],
    ) -> Result<(Model, Vec<labels::Model>)> {
        if title.trim().is_empty() {
            return bad_request("issue title must not be empty");
        }

        let txn = db.begin().await?;
        let last = Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .order_by_desc(Column::Number)
            .one(&txn)
            .await?;
        let number = last.map_or(1, |issue| issue.number + 1);

        let issue = ActiveModel {
            repository_id: Set(repository_id),
            user_id: Set(user_id),
            number: Set(number),
            title: Set(title.trim().to_owned()),
            body: Set(body.map(str::to_owned)),
            state: Set(STATE_OPEN.to_owned()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        let attached = if label_ids.is_empty() {
            Vec::new()
        } else {
            let found = labels::Entity::find()
                .filter(labels::Column::RepositoryId.eq(repository_id))
                .filter(labels::Column::Id.is_in(label_ids.iter().copied()))
                .all(&txn)
                .await?;
            if found.len() != label_ids.len() {
                return bad_request("label does not belong to this repository");
            }
            for label_id in label_ids {
                issue_labels::ActiveModel {
                    issue_id: Set(issue.id),
                    label_id: Set(*label_id),
                    ..Default::default()
                }
                .insert(&txn)
                .await?;
            }
            found
        };

        txn.commit().await?;
        Ok((issue, attached))
    }

    /// Transitions the issue to open or closed.
    ///
    /// # Errors
    /// Returns a database error if the update fails.
    pub async fn set_state(mut self, db: &DatabaseConnection, state: &str) -> Result<Model> {
        self.state = Set(state.to_owned());
        self.update(db).await.map_err(Into::into)
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
