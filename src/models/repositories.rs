//! Repository domain model and the path-safety rules that go with it.
//!
//! A repository has two representations that must be kept in step:
//!
//! * a row in `repositories` — metadata, owner, visibility (the source of truth
//!   for *who may do what*);
//! * a bare git repository on disk under `settings.repo_base_dir` (the source
//!   of truth for *what is in it*).
//!
//! Everything that turns user input into a filesystem path goes through
//! [`validate_name`], [`validate_owner`] and [`repo_dir`]. Keeping those checks
//! here — rather than in each handler — is what stops a clone URL from
//! escaping the repo root.

use super::_entities::repositories::Column;
pub use super::_entities::repositories::{ActiveModel, Entity, Model};
use super::{collaborators, users};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;
use std::path::PathBuf;

pub type Repositories = Entity;

/// Longest accepted repository name.
pub const NAME_MAX_LEN: usize = 100;
/// GitHub-style owner handle limit.
pub const OWNER_MAX_LEN: usize = 39;

/// Shared primitive behind the owner and repository-name validators.
///
/// Allows ASCII letters, digits, `-`, `_` and `.`, but rejects leading dots,
/// trailing dots and any `..` sequence — those are the ingredients of a path
/// traversal attack when the value is later joined onto the repo root.
fn is_safe_segment(value: &str, max_len: usize) -> bool {
    if value.is_empty() || value.len() > max_len {
        return false;
    }
    let first = value.chars().next().unwrap_or('.');
    let last = value.chars().last().unwrap_or('.');
    if matches!(first, '.' | '-' | '_') || matches!(last, '.' | '-' | '_') || value.contains("..") {
        return false;
    }
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Validates the bare repository name a user typed (without the `.git` suffix).
///
/// # Errors
/// Returns a 400 error when the name is empty, too long, contains path
/// separators or traversal sequences, or already carries a `.git` suffix.
pub fn validate_name(name: &str) -> Result<()> {
    if !is_safe_segment(name, NAME_MAX_LEN) || name.ends_with(".git") {
        return bad_request("invalid repository name");
    }
    Ok(())
}

/// Validates the owner handle taken from a URL path segment.
///
/// # Errors
/// Returns a 400 error when the handle could be a path component rather than
/// a username.
pub fn validate_owner(owner: &str) -> Result<()> {
    if !is_safe_segment(owner, OWNER_MAX_LEN) {
        return bad_request("invalid owner name");
    }
    Ok(())
}

/// Builds the on-disk path of a bare repository.
///
/// The caller's `base`, `owner` and `name` are all validated before they are
/// joined, so the result can never walk outside `base`.
///
/// # Errors
/// Returns an error for an invalid owner/name or an empty base directory.
pub fn repo_dir(base: &str, owner: &str, name: &str) -> Result<PathBuf> {
    validate_owner(owner)?;
    validate_name(name)?;
    if base.trim().is_empty() {
        return Err(Error::string("repo_base_dir must not be empty"));
    }
    Ok(PathBuf::from(base).join(owner).join(format!("{name}.git")))
}

/// Roles that can be granted to a collaborator. The owner is implicit and is
/// not stored in the `collaborators` table.
pub const ROLE_READ: &str = "read";
pub const ROLE_WRITE: &str = "write";
pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_OWNER: &str = "owner";

/// The actions a user may take on a repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoAction {
    /// Clone, fetch and browse code, issues and pull requests.
    Read,
    /// Push, and create/merge issues and pull requests.
    Write,
    /// Manage collaborators and repository settings.
    Admin,
}

impl RepoAction {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Admin => "admin",
        }
    }
}

/// Whether a collaborator role grants an action. The owner bypasses this.
#[must_use]
pub fn role_allows(role: &str, action: RepoAction) -> bool {
    match role {
        ROLE_ADMIN => true,
        ROLE_WRITE => matches!(action, RepoAction::Read | RepoAction::Write),
        ROLE_READ => action == RepoAction::Read,
        _ => false,
    }
}

/// Whether a string is a grantable collaborator role.
#[must_use]
pub fn is_collaborator_role(role: &str) -> bool {
    matches!(role, ROLE_READ | ROLE_WRITE | ROLE_ADMIN)
}

/// Read-oriented domain logic.
impl Model {
    /// Whether `user_id` owns this repository.
    #[must_use]
    pub fn is_owned_by(&self, user_id: i64) -> bool {
        self.user_id == user_id
    }

    /// The single authorization rule used by REST handlers *and* git transport.
    ///
    /// The owner may do everything. Public repositories grant `Read` to anyone.
    /// Other users are checked against their collaborator role.
    ///
    /// # Errors
    /// Returns a database error if the collaborator lookup fails.
    pub async fn can(
        &self,
        db: &DatabaseConnection,
        user_id: Option<i64>,
        action: RepoAction,
    ) -> Result<bool> {
        if user_id == Some(self.user_id) {
            return Ok(true);
        }
        if action == RepoAction::Read && !self.is_private() {
            return Ok(true);
        }
        let Some(user_id) = user_id else {
            return Ok(false);
        };
        let collaborator = collaborators::Entity::find_for(db, self.id, user_id).await?;
        Ok(collaborator.is_some_and(|row| role_allows(&row.role, action)))
    }

    /// Convenience wrapper for [`Self::can`] with [`RepoAction::Read`].
    ///
    /// # Errors
    /// Returns a database error if the collaborator lookup fails.
    pub async fn can_read(&self, db: &DatabaseConnection, viewer_id: Option<i64>) -> Result<bool> {
        self.can(db, viewer_id, RepoAction::Read).await
    }

    /// Convenience wrapper for [`Self::can`] with [`RepoAction::Write`].
    ///
    /// # Errors
    /// Returns a database error if the collaborator lookup fails.
    pub async fn can_write(&self, db: &DatabaseConnection, user_id: i64) -> Result<bool> {
        self.can(db, Some(user_id), RepoAction::Write).await
    }

    /// Convenience wrapper for [`Self::can`] with [`RepoAction::Admin`].
    ///
    /// # Errors
    /// Returns a database error if the collaborator lookup fails.
    pub async fn can_admin(&self, db: &DatabaseConnection, user_id: i64) -> Result<bool> {
        self.can(db, Some(user_id), RepoAction::Admin).await
    }

    /// Highest role a user has in this repository, for display in the SPA.
    /// Anonymous viewers get `"none"`.
    ///
    /// # Errors
    /// Returns a database error if the collaborator lookup fails.
    pub async fn role_for(&self, db: &DatabaseConnection, user_id: Option<i64>) -> Result<String> {
        let Some(user_id) = user_id else {
            return Ok("none".to_owned());
        };
        if self.is_owned_by(user_id) {
            return Ok(ROLE_OWNER.to_owned());
        }
        let role = collaborators::Entity::find_for(db, self.id, user_id)
            .await?
            .map(|row| row.role)
            .unwrap_or_else(|| "none".to_owned());
        Ok(role)
    }

    /// The repository name as stored, or an error if a legacy row has none.
    ///
    /// # Errors
    /// Returns an error when the `name` column is null.
    pub fn display_name(&self) -> Result<&str> {
        self.name
            .as_deref()
            .ok_or_else(|| Error::string("repository name is missing"))
    }

    /// The repository visibility, defaulting to private when a legacy row is
    /// null. Failing closed is the safe choice here.
    #[must_use]
    pub fn is_private(&self) -> bool {
        self.is_private.unwrap_or(true)
    }

    /// Path component of the clone URL, e.g. `/alice/hello.git`.
    ///
    /// # Errors
    /// Returns an error when the repository name is null.
    pub fn clone_path(&self, owner: &str) -> Result<String> {
        Ok(format!("/{owner}/{}.git", self.display_name()?))
    }

    /// All repositories owned by a user.
    ///
    /// # Errors
    /// Returns a model error if the query fails.
    pub async fn find_by_user(db: &DatabaseConnection, user_id: i64) -> ModelResult<Vec<Self>> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// Look up a repository by its `owner/name` pair.
    ///
    /// The owner is resolved to a user first; the repository is then matched
    /// by owner id *and* name so two users can each have a `hello` repo.
    ///
    /// # Errors
    /// Returns `EntityNotFound` if either the user or the repository is absent.
    pub async fn find_by_owner_and_name(
        db: &DatabaseConnection,
        owner: &str,
        name: &str,
    ) -> ModelResult<Self> {
        let user = users::Model::find_by_username(db, owner).await?;
        Entity::find()
            .filter(Column::UserId.eq(user.id))
            .filter(Column::Name.eq(name))
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Inserts a new repository after validating its name.
    ///
    /// # Errors
    /// Returns a 400 error for an invalid name, or a database error on insert.
    pub async fn create_for_user(
        db: &DatabaseConnection,
        user_id: i64,
        name: &str,
        description: Option<&str>,
        is_private: bool,
    ) -> Result<Model> {
        validate_name(name)?;
        ActiveModel {
            name: Set(Some(name.to_owned())),
            description: Set(description.map(str::to_owned)),
            is_private: Set(Some(is_private)),
            user_id: Set(user_id),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_repository_names() {
        for name in [
            "hello-world",
            "my_repo",
            "a.b",
            "ABC123",
            &"a".repeat(NAME_MAX_LEN),
        ] {
            assert!(validate_name(name).is_ok(), "expected {name:?} to be valid");
        }
    }

    #[test]
    fn rejects_names_that_could_escape_the_repo_root() {
        for name in [
            "",
            ".",
            "..",
            "../secret",
            "a/b",
            "a\\b",
            "a..b",
            "with space",
            "-leading",
            "trailing-",
            "repo.git",
            &"a".repeat(NAME_MAX_LEN + 1),
        ] {
            assert!(
                validate_name(name).is_err(),
                "expected {name:?} to be rejected"
            );
        }
    }

    #[test]
    fn owner_validation_follows_the_same_rules_with_a_shorter_limit() {
        assert!(validate_owner("alice").is_ok());
        assert!(validate_owner("a_b-1").is_ok());
        assert!(validate_owner("").is_err());
        assert!(validate_owner("../alice").is_err());
        assert!(validate_owner(&"a".repeat(OWNER_MAX_LEN + 1)).is_err());
    }

    #[test]
    fn collaborator_roles_grant_the_expected_actions() {
        assert!(role_allows(ROLE_READ, RepoAction::Read));
        assert!(!role_allows(ROLE_READ, RepoAction::Write));
        assert!(role_allows(ROLE_WRITE, RepoAction::Write));
        assert!(!role_allows(ROLE_WRITE, RepoAction::Admin));
        assert!(role_allows(ROLE_ADMIN, RepoAction::Admin));
        assert!(!role_allows("bogus", RepoAction::Read));
        assert!(is_collaborator_role(ROLE_ADMIN));
        assert!(!is_collaborator_role("owner"));
    }

    #[test]
    fn repo_dir_joins_validated_components() {
        let path = repo_dir("/var/repos", "alice", "hello").unwrap();
        assert_eq!(path, PathBuf::from("/var/repos/alice/hello.git"));
    }

    #[test]
    fn repo_dir_refuses_path_traversal() {
        assert!(repo_dir("/var/repos", "../etc", "hello").is_err());
        assert!(repo_dir("/var/repos", "alice", "../hello").is_err());
        assert!(repo_dir("", "alice", "hello").is_err());
    }
}
