//! Teams: groups of org members granted roles on selected repositories.

pub use super::_entities::teams::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type Teams = Entity;

/// Read-oriented domain logic.
impl Entity {
    /// All teams in an organization, oldest first.
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

    /// A team by id, scoped to an organization so one org cannot reach another's.
    ///
    /// # Errors
    /// Returns `EntityNotFound` if the team is absent or belongs elsewhere.
    pub async fn find_in_organization(
        db: &DatabaseConnection,
        organization_id: i64,
        team_id: i64,
    ) -> ModelResult<Model> {
        Entity::find()
            .filter(Column::Id.eq(team_id))
            .filter(Column::OrganizationId.eq(organization_id))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Creates a team; names are unique within an organization.
    ///
    /// # Errors
    /// Returns a 400 for an invalid/duplicate name, or a database error.
    pub async fn create(
        db: &DatabaseConnection,
        organization_id: i64,
        name: &str,
        description: Option<&str>,
    ) -> Result<Model> {
        let name = name.trim();
        if name.is_empty() || name.len() > 100 {
            return bad_request("invalid team name");
        }
        if Entity::find()
            .filter(Column::OrganizationId.eq(organization_id))
            .filter(Column::Name.eq(name))
            .one(db)
            .await?
            .is_some()
        {
            return bad_request("a team with that name already exists");
        }
        ActiveModel {
            organization_id: Set(organization_id),
            name: Set(name.to_owned()),
            description: Set(description.map(str::to_owned)),
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
