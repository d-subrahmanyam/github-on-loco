//! Pull request domain model: per-repository numbering, state and merge info.
//!
//! A PR is opened against a *base* branch in the same repository. The head is
//! stored as both a ref (for display) and an immutable SHA (what actually gets
//! merged), so a later force-push cannot silently change the reviewed diff.

pub use super::_entities::pull_requests::Column;
pub use super::_entities::pull_requests::{ActiveModel, Entity, Model};
use super::review_comments;
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;
use sea_orm::TransactionTrait;

pub type PullRequests = Entity;

/// State values stored in the `state` column.
pub const STATE_OPEN: &str = "open";
pub const STATE_CLOSED: &str = "closed";
pub const STATE_MERGED: &str = "merged";

/// Read-oriented domain logic.
impl Model {
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.state == STATE_OPEN
    }

    #[must_use]
    pub fn is_merged(&self) -> bool {
        self.state == STATE_MERGED
    }

    /// PRs in a repository, optionally filtered by state.
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
            if [STATE_OPEN, STATE_CLOSED, STATE_MERGED].contains(&state) {
                query = query.filter(Column::State.eq(state));
            }
        }
        query
            .order_by_desc(Column::Number)
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// Look up a PR by its per-repository number.
    ///
    /// # Errors
    /// Returns `EntityNotFound` when the repository has no such PR.
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

    /// Number of review comments on this PR.
    ///
    /// # Errors
    /// Returns a database error if the count fails.
    pub async fn comment_count(&self, db: &DatabaseConnection) -> Result<i64> {
        Ok(review_comments::Entity::find()
            .filter(review_comments::Column::PullRequestId.eq(self.id))
            .count(db)
            .await? as i64)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Creates a PR and allocates its number atomically.
    ///
    /// The highest existing number is read inside the same transaction as the
    /// insert, so two concurrent creations cannot claim the same number.
    ///
    /// # Errors
    /// Returns a 400 for an empty title or a head equal to the base branch.
    // Every field below is a required part of a PR; grouping them in a struct
    // would only shift the boilerplate to each caller.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_for_repository(
        db: &DatabaseConnection,
        repository_id: i64,
        user_id: i64,
        title: &str,
        body: Option<&str>,
        head_ref: &str,
        head_sha: &str,
        base_ref: &str,
    ) -> Result<Model> {
        if title.trim().is_empty() {
            return bad_request("pull request title must not be empty");
        }
        if head_ref == base_ref {
            return bad_request("head and base branches must differ");
        }

        let txn = db.begin().await?;
        let last = Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .order_by_desc(Column::Number)
            .one(&txn)
            .await?;
        let number = last.map_or(1, |pr| pr.number + 1);

        let pr = ActiveModel {
            repository_id: Set(repository_id),
            user_id: Set(user_id),
            number: Set(number),
            title: Set(title.trim().to_owned()),
            body: Set(body.map(str::to_owned)),
            state: Set(STATE_OPEN.to_owned()),
            head_ref: Set(head_ref.to_owned()),
            head_sha: Set(head_sha.to_owned()),
            base_ref: Set(base_ref.to_owned()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;
        Ok(pr)
    }

    /// Transitions the PR to open or closed.
    ///
    /// # Errors
    /// Returns a database error if the update fails.
    pub async fn set_state(mut self, db: &DatabaseConnection, state: &str) -> Result<Model> {
        self.state = Set(state.to_owned());
        self.update(db).await.map_err(Into::into)
    }

    /// Records a successful merge: state, resulting commit and timestamp.
    ///
    /// # Errors
    /// Returns a database error if the update fails.
    pub async fn mark_merged(mut self, db: &DatabaseConnection, commit_sha: &str) -> Result<Model> {
        self.state = Set(STATE_MERGED.to_owned());
        self.merged_commit_sha = Set(Some(commit_sha.to_owned()));
        self.merged_at = Set(Some(chrono::Utc::now().into()));
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
