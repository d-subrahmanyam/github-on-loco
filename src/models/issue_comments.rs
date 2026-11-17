//! Comments attached to an issue.

pub use super::_entities::issue_comments::Column;
pub use super::_entities::issue_comments::{ActiveModel, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type IssueComments = Entity;

/// Read-oriented domain logic.
impl Entity {
    /// All comments on an issue, oldest first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_issue(db: &DatabaseConnection, issue_id: i64) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::IssueId.eq(issue_id))
            .order_by_asc(Column::Id)
            .all(db)
            .await
            .map_err(Into::into)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Adds a comment to an issue after validating that it is not blank.
    ///
    /// # Errors
    /// Returns a 400 for an empty body, or a database error on insert.
    pub async fn create_for_issue(
        db: &DatabaseConnection,
        issue_id: i64,
        user_id: i64,
        body: &str,
    ) -> Result<Model> {
        if body.trim().is_empty() {
            return bad_request("comment must not be empty");
        }
        ActiveModel {
            issue_id: Set(issue_id),
            user_id: Set(user_id),
            body: Set(body.trim().to_owned()),
            ..Default::default()
        }
        .insert(db)
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
