//! Organization membership grants.

pub use super::_entities::org_members::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

use super::organizations;

pub type OrgMembers = Entity;

/// Read-oriented domain logic.
impl Entity {
    /// A user's role in an organization, if they are a member.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn role_for(
        db: &DatabaseConnection,
        organization_id: i64,
        user_id: i64,
    ) -> ModelResult<Option<String>> {
        Ok(Self::find_for(db, organization_id, user_id)
            .await?
            .map(|row| row.role))
    }

    /// One membership row, if it exists.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_for<C: ConnectionTrait>(
        db: &C,
        organization_id: i64,
        user_id: i64,
    ) -> ModelResult<Option<Model>> {
        Entity::find()
            .filter(Column::OrganizationId.eq(organization_id))
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await
            .map_err(Into::into)
    }

    /// Every member of an organization, oldest first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_organization(
        db: &DatabaseConnection,
        organization_id: i64,
    ) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::OrganizationId.eq(organization_id))
            .order_by_asc(Column::Id)
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// All organizations a user belongs to.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_user(db: &DatabaseConnection, user_id: i64) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::UserId.eq(user_id))
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// Number of owners, used to prevent an org being left with none.
    ///
    /// # Errors
    /// Returns a database error if the count fails.
    pub async fn owner_count(db: &DatabaseConnection, organization_id: i64) -> Result<i64> {
        Ok(Entity::find()
            .filter(Column::OrganizationId.eq(organization_id))
            .filter(Column::Role.eq(organizations::ROLE_OWNER))
            .count(db)
            .await? as i64)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Adds a member. Repeated calls are a no-op.
    ///
    /// # Errors
    /// Returns a 400 for an invalid role, or a database error.
    pub async fn add<C: ConnectionTrait>(
        db: &C,
        organization_id: i64,
        user_id: i64,
        role: &str,
    ) -> Result<Model> {
        if !organizations::is_member_role(role) {
            return bad_request("invalid organization role");
        }
        if let Some(existing) = Entity::find_for(db, organization_id, user_id).await? {
            return Ok(existing);
        }
        ActiveModel {
            organization_id: Set(organization_id),
            user_id: Set(user_id),
            role: Set(role.to_owned()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(Into::into)
    }

    /// Changes a member's role. Refuses to demote the last owner.
    ///
    /// # Errors
    /// Returns a 400 for an invalid role or the last-owner case, or a db error.
    pub async fn set_role(
        db: &DatabaseConnection,
        organization_id: i64,
        user_id: i64,
        role: &str,
    ) -> Result<Model> {
        if !organizations::is_member_role(role) {
            return bad_request("invalid organization role");
        }
        let existing = Entity::find_for(db, organization_id, user_id)
            .await?
            .ok_or_else(|| Error::string("user is not a member"))?;
        if existing.role == organizations::ROLE_OWNER
            && role != organizations::ROLE_OWNER
            && Entity::owner_count(db, organization_id).await? <= 1
        {
            return bad_request("an organization must have at least one owner");
        }
        let mut active: ActiveModel = existing.into();
        active.role = Set(role.to_owned());
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(db).await.map_err(Into::into)
    }

    /// Removes a member. Refuses to remove the last owner.
    ///
    /// # Errors
    /// Returns a 400 for the last-owner case, or a database error.
    pub async fn remove(db: &DatabaseConnection, organization_id: i64, user_id: i64) -> Result<()> {
        let Some(existing) = Entity::find_for(db, organization_id, user_id).await? else {
            return Ok(());
        };
        if existing.role == organizations::ROLE_OWNER
            && Entity::owner_count(db, organization_id).await? <= 1
        {
            return bad_request("an organization must have at least one owner");
        }
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
