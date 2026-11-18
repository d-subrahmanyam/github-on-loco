//! Pull request reviews: one verdict per reviewer, upserted when they change
//! their mind. Approving reviews are what branch protection counts.

pub use super::_entities::pr_reviews::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type PrReviews = Entity;

/// Review verdicts. `commented` leaves the approval count unchanged.
pub mod state {
    pub const APPROVED: &str = "approved";
    pub const CHANGES_REQUESTED: &str = "changes_requested";
    pub const COMMENTED: &str = "commented";
}

/// Whether a string is an accepted review verdict.
#[must_use]
pub fn is_review_state(state: &str) -> bool {
    matches!(
        state,
        state::APPROVED | state::CHANGES_REQUESTED | state::COMMENTED
    )
}

impl Model {
    /// Whether this review is an approval.
    #[must_use]
    pub fn is_approval(&self) -> bool {
        self.state == state::APPROVED
    }
}

/// Read-oriented domain logic.
impl Entity {
    /// All reviews on a PR, oldest first.
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

    /// A single reviewer's current verdict on a PR.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_for(
        db: &DatabaseConnection,
        pull_request_id: i64,
        user_id: i64,
    ) -> ModelResult<Option<Model>> {
        Entity::find()
            .filter(Column::PullRequestId.eq(pull_request_id))
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await
            .map_err(Into::into)
    }

    /// Number of distinct approving reviewers, excluding the PR author so a
    /// self-approval can never satisfy a rule (even on legacy rows).
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn approval_count(
        db: &DatabaseConnection,
        pull_request_id: i64,
        author_id: i64,
    ) -> Result<i64> {
        Ok(Entity::find()
            .filter(Column::PullRequestId.eq(pull_request_id))
            .filter(Column::State.eq(state::APPROVED))
            .filter(Column::UserId.ne(author_id))
            .count(db)
            .await? as i64)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Records or replaces the caller's review. A reviewer gets one row per PR;
    /// changing `commented` → `approved` updates it in place.
    ///
    /// # Errors
    /// Returns a 400 for an invalid verdict, or a database error.
    pub async fn set_review(
        db: &DatabaseConnection,
        pull_request_id: i64,
        user_id: i64,
        state: &str,
        body: Option<&str>,
    ) -> Result<Model> {
        if !is_review_state(state) {
            return bad_request("invalid review state");
        }
        let body = body.map(str::trim).filter(|text| !text.is_empty());
        if let Some(existing) = Entity::find_for(db, pull_request_id, user_id).await? {
            let mut active: ActiveModel = existing.into();
            active.state = Set(state.to_owned());
            active.body = Set(body.map(str::to_owned));
            active.updated_at = Set(chrono::Utc::now().into());
            active.update(db).await.map_err(Into::into)
        } else {
            ActiveModel {
                pull_request_id: Set(pull_request_id),
                user_id: Set(user_id),
                state: Set(state.to_owned()),
                body: Set(body.map(str::to_owned)),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(Into::into)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_three_verdicts_are_accepted() {
        assert!(is_review_state(state::APPROVED));
        assert!(is_review_state(state::CHANGES_REQUESTED));
        assert!(is_review_state(state::COMMENTED));
        assert!(!is_review_state("lgtm"));
    }
}
