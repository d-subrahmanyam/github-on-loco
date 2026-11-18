//! Organizations: a second kind of repository owner.
//!
//! An org has members with `owner`/`member` roles and owns teams. Repository
//! ownership is recorded on `repositories.organization_id`; `user_id` stays as
//! the creator for audit. The org owner is the only role that can manage the
//! organization itself.

pub use super::_entities::organizations::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

use super::{org_members, repositories, users};

pub type Organizations = Entity;

pub const ROLE_OWNER: &str = "owner";
pub const ROLE_MEMBER: &str = "member";

/// Whether a string is a grantable org membership role.
#[must_use]
pub fn is_member_role(role: &str) -> bool {
    matches!(role, ROLE_OWNER | ROLE_MEMBER)
}

impl Model {
    /// Whether the user is an owner of this organization.
    ///
    /// # Errors
    /// Returns a database error if the membership lookup fails.
    pub async fn is_owner(&self, db: &DatabaseConnection, user_id: i64) -> Result<bool> {
        Ok(org_members::Entity::role_for(db, self.id, user_id).await?
            == Some(ROLE_OWNER.to_owned()))
    }

    /// Highest org role for display. Owners see `"owner"`, members `"member"`.
    ///
    /// # Errors
    /// Returns a database error if the lookup fails.
    pub async fn role_for(&self, db: &DatabaseConnection, user_id: i64) -> Result<Option<String>> {
        Ok(org_members::Entity::role_for(db, self.id, user_id).await?)
    }
}

/// Read-oriented domain logic.
impl Entity {
    /// Finds an organization by its public handle.
    ///
    /// # Errors
    /// Returns `EntityNotFound` when the handle is unknown.
    pub async fn find_by_handle(db: &DatabaseConnection, handle: &str) -> ModelResult<Model> {
        Entity::find()
            .filter(Column::Handle.eq(handle))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)
    }

    /// Organizations a user belongs to, oldest first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_user(db: &DatabaseConnection, user_id: i64) -> ModelResult<Vec<Model>> {
        let memberships = org_members::Entity::find_by_user(db, user_id).await?;
        let ids: Vec<i64> = memberships.iter().map(|row| row.organization_id).collect();
        Entity::find()
            .filter(Column::Id.is_in(ids))
            .order_by_asc(Column::Id)
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// All organizations, for the public directory.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_all(db: &DatabaseConnection) -> ModelResult<Vec<Model>> {
        Entity::find()
            .order_by_asc(Column::Id)
            .all(db)
            .await
            .map_err(Into::into)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Creates an organization and makes the creator its first owner. The two
    /// writes share a transaction so the org can never have zero owners.
    ///
    /// # Errors
    /// Returns a 400 for an invalid handle or duplicate, or a database error.
    pub async fn create_with_owner(
        db: &DatabaseConnection,
        owner_id: i64,
        handle: &str,
        display_name: &str,
        description: Option<&str>,
    ) -> Result<Model> {
        // The handle is used as a URL segment and as the on-disk repo directory.
        repositories::validate_owner(handle)?;
        if handle.trim().is_empty() || display_name.trim().is_empty() {
            return bad_request("handle and display name must not be empty");
        }
        if Entity::find()
            .filter(Column::Handle.eq(handle))
            .one(db)
            .await?
            .is_some()
        {
            return bad_request("that handle is already taken");
        }
        let transaction = db.begin().await?;
        let org = ActiveModel {
            handle: Set(handle.to_owned()),
            display_name: Set(display_name.trim().to_owned()),
            description: Set(description.map(str::to_owned)),
            ..Default::default()
        }
        .insert(&transaction)
        .await?;
        org_members::ActiveModel::add(&transaction, org.id, owner_id, ROLE_OWNER).await?;
        transaction.commit().await?;
        Ok(org)
    }
}

/// Helpers for rendering the org owner in repo URLs.
impl Model {
    /// Canonical public handle, mirroring `users::Model::owner_handle`.
    #[must_use]
    pub fn owner_handle(&self) -> &str {
        &self.handle
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

/// Resolves a user id to a handle, for member listings. Kept here so the
/// controller does not reach into `users::_entities`.
pub async fn member_username(db: &DatabaseConnection, user_id: i64) -> Result<String> {
    Ok(users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .map(|user| user.username)
        .unwrap_or_else(|| "unknown".to_owned()))
}
