//! Branch protection rules: per-branch requirements that constrain how changes
//! may land on a repository branch.
//!
//! Two rules are enforced today:
//!
//! * `require_pull_request` — direct pushes to the branch are rejected by the
//!   git transport; the only way in is a pull request merge (which uses git
//!   plumbing directly, bypassing the HTTP push path). Repository admins may
//!   still push directly, matching GitHub's default "admins exempt" behaviour.
//! * `required_approvals` — the merge endpoint refuses to enqueue a merge until
//!   the PR has at least this many approving reviews.
//!
//! Rules are settings, not permissions: only their interaction with the single
//! `repositories::Model::can` rule decides who may change them or bypass them.

pub use super::_entities::branch_protection_rules::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type BranchProtectionRules = Entity;

/// Upper bound accepted by the API, so a typo cannot lock a branch forever.
pub const MAX_REQUIRED_APPROVALS: i64 = 6;

impl Model {
    /// Whether this rule blocks direct pushes.
    #[must_use]
    pub fn require_pull_request(&self) -> bool {
        self.require_pull_request
    }

    /// Number of approving reviews a PR targeting this branch needs.
    #[must_use]
    pub fn required_approvals(&self) -> i64 {
        self.required_approvals
    }
}

/// Validates a branch name used in a protection rule. This is a ref name, not a
/// filesystem path, but the same caution applies: reject empty names, traversal
/// sequences and whitespace so rules stay unambiguous and render safely.
///
/// # Errors
/// Returns a 400 error for an unusable branch name.
pub fn validate_branch(branch: &str) -> Result<()> {
    let branch = branch.trim();
    if branch.is_empty()
        || branch.len() > 255
        || branch.starts_with('/')
        || branch.ends_with('/')
        || branch.contains("..")
        || branch.contains("//")
        || branch.chars().any(char::is_whitespace)
        || branch.chars().any(char::is_control)
    {
        return bad_request("invalid branch name");
    }
    Ok(())
}

/// Read-oriented domain logic.
impl Entity {
    /// Looks up the rule for one branch of a repository, if any.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_for_branch(
        db: &DatabaseConnection,
        repository_id: i64,
        branch: &str,
    ) -> ModelResult<Option<Model>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .filter(Column::Branch.eq(branch))
            .one(db)
            .await
            .map_err(Into::into)
    }

    /// Every rule configured on a repository, oldest first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_repository(
        db: &DatabaseConnection,
        repository_id: i64,
    ) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .order_by_asc(Column::Id)
            .all(db)
            .await
            .map_err(Into::into)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Creates or updates the rule for a branch. Upserting keeps the settings
    /// page idempotent and the unique `(repository_id, branch)` index intact.
    ///
    /// # Errors
    /// Returns a 400 for an invalid branch/approval count, or a database error.
    pub async fn upsert(
        db: &DatabaseConnection,
        repository_id: i64,
        branch: &str,
        require_pull_request: bool,
        required_approvals: i64,
    ) -> Result<Model> {
        validate_branch(branch)?;
        if !(0..=MAX_REQUIRED_APPROVALS).contains(&required_approvals) {
            return bad_request("required approvals must be between 0 and 6");
        }
        let branch = branch.trim();
        if let Some(existing) = Entity::find_for_branch(db, repository_id, branch).await? {
            let mut active: ActiveModel = existing.into();
            active.require_pull_request = Set(require_pull_request);
            active.required_approvals = Set(required_approvals);
            active.updated_at = Set(chrono::Utc::now().into());
            active.update(db).await.map_err(Into::into)
        } else {
            ActiveModel {
                repository_id: Set(repository_id),
                branch: Set(branch.to_owned()),
                require_pull_request: Set(require_pull_request),
                required_approvals: Set(required_approvals),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(Into::into)
        }
    }

    /// Removes a rule. Deleting a rule that does not exist is a no-op.
    ///
    /// # Errors
    /// Returns a database error if the delete fails.
    pub async fn remove(
        db: &DatabaseConnection,
        repository_id: i64,
        branch: &str,
    ) -> ModelResult<()> {
        let Some(existing) = Entity::find_for_branch(db, repository_id, branch).await? else {
            return Ok(());
        };
        let active: ActiveModel = existing.into();
        active.delete(db).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_realistic_branch_names() {
        for branch in ["main", "release/1.2", "feature/x-y_z"] {
            assert!(validate_branch(branch).is_ok(), "{branch} should be valid");
        }
    }

    #[test]
    fn rejects_ambiguous_branch_names() {
        for branch in [
            "",
            "  ",
            "..",
            "a/../b",
            "a//b",
            "/main",
            "main/",
            "has space",
        ] {
            assert!(
                validate_branch(branch).is_err(),
                "{branch} should be rejected"
            );
        }
    }
}
