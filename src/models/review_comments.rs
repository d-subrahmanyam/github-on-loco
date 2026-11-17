//! Review comments on a pull request. A comment may be general (`path` and
//! `line` are null) or anchored to a line in the compared diff.

pub use super::_entities::review_comments::Column;
pub use super::_entities::review_comments::{ActiveModel, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type ReviewComments = Entity;

/// Read-oriented domain logic.
impl Model {}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Adds a review comment, rejecting an empty body.
    ///
    /// # Errors
    /// Returns a 400 for an empty body, or a database error.
    pub async fn create_for_pr(
        db: &DatabaseConnection,
        pull_request_id: i64,
        user_id: i64,
        body: &str,
        path: Option<&str>,
        line: Option<i64>,
    ) -> Result<Model> {
        if body.trim().is_empty() {
            return bad_request("review comment must not be empty");
        }
        ActiveModel {
            pull_request_id: Set(pull_request_id),
            user_id: Set(user_id),
            body: Set(body.trim().to_owned()),
            path: Set(path.map(str::to_owned)),
            line: Set(line),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(Into::into)
    }
}

/// Custom finders.
impl Entity {
    /// All review comments on a PR, oldest first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_pr(
        db: &DatabaseConnection,
        pull_request_id: i64,
    ) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::PullRequestId.eq(pull_request_id))
            .order_by_asc(Column::Id)
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
