//! Team membership. Only organization members may be added to a team.

pub use super::_entities::team_members::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

use super::org_members;

pub type TeamMembers = Entity;

/// Read-oriented domain logic.
impl Entity {
    /// All members of a team, oldest first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_team(db: &DatabaseConnection, team_id: i64) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::TeamId.eq(team_id))
            .order_by_asc(Column::Id)
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// Number of members in a team.
    ///
    /// # Errors
    /// Returns a database error if the count fails.
    pub async fn count_for_team(db: &DatabaseConnection, team_id: i64) -> Result<i64> {
        Ok(Entity::find()
            .filter(Column::TeamId.eq(team_id))
            .count(db)
            .await? as i64)
    }

    /// Whether a user is in a team.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn contains(db: &DatabaseConnection, team_id: i64, user_id: i64) -> Result<bool> {
        Ok(Entity::find()
            .filter(Column::TeamId.eq(team_id))
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await?
            .is_some())
    }

    /// All team ids a user belongs to in an organization.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn team_ids_for_user(
        db: &DatabaseConnection,
        organization_id: i64,
        user_id: i64,
    ) -> Result<Vec<i64>> {
        use super::teams;
        let rows = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .all(db)
            .await?;
        let team_ids: Vec<i64> = rows.iter().map(|row| row.team_id).collect();
        if team_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(teams::Entity::find()
            .filter(teams::Column::Id.is_in(team_ids))
            .filter(teams::Column::OrganizationId.eq(organization_id))
            .all(db)
            .await?
            .iter()
            .map(|team| team.id)
            .collect())
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Adds a user to a team, provided they are a member of its organization.
    ///
    /// # Errors
    /// Returns a 400 when the user is not an org member, or a database error.
    pub async fn add(
        db: &DatabaseConnection,
        team: &super::teams::Model,
        user_id: i64,
    ) -> Result<Model> {
        if org_members::Entity::find_for(db, team.organization_id, user_id)
            .await?
            .is_none()
        {
            return bad_request("only organization members can join a team");
        }
        if Entity::contains(db, team.id, user_id).await? {
            return Entity::find()
                .filter(Column::TeamId.eq(team.id))
                .filter(Column::UserId.eq(user_id))
                .one(db)
                .await?
                .ok_or_else(|| Error::string("team member disappeared"));
        }
        ActiveModel {
            team_id: Set(team.id),
            user_id: Set(user_id),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(Into::into)
    }

    /// Removes a user from a team. Repeated calls are a no-op.
    ///
    /// # Errors
    /// Returns a database error if the delete fails.
    pub async fn remove(db: &DatabaseConnection, team_id: i64, user_id: i64) -> ModelResult<()> {
        let found = Entity::find()
            .filter(Column::TeamId.eq(team_id))
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await?;
        if let Some(row) = found {
            let active: ActiveModel = row.into();
            active.delete(db).await?;
        }
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
