//! Team grants: the role a team has on one organization repository.

pub use super::_entities::team_repositories::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

use super::{repositories, teams};

pub type TeamRepositories = Entity;

/// Read-oriented domain logic.
impl Entity {
    /// All grants on a team's repositories.
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

    /// Grants relevant to a user on a repository. The caller filters team ids.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_for_repository_and_teams(
        db: &DatabaseConnection,
        repository_id: i64,
        team_ids: &[i64],
    ) -> Result<Vec<Model>> {
        if team_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .filter(Column::TeamId.is_in(team_ids.to_vec()))
            .all(db)
            .await?)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Creates or updates a team's role on a repository.
    ///
    /// # Errors
    /// Returns a 400 for an invalid role or a foreign repository, or a db error.
    pub async fn grant(
        db: &DatabaseConnection,
        team: &teams::Model,
        repository_id: i64,
        role: &str,
    ) -> Result<Model> {
        if !repositories::is_collaborator_role(role) {
            return bad_request("invalid team role");
        }
        let repo = repositories::Entity::find_by_id(repository_id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        if repo.organization_id != Some(team.organization_id) {
            return bad_request(
                "a team can only be granted roles on its own organization's repositories",
            );
        }
        if let Some(existing) = Entity::find()
            .filter(Column::TeamId.eq(team.id))
            .filter(Column::RepositoryId.eq(repository_id))
            .one(db)
            .await?
        {
            let mut active: ActiveModel = existing.into();
            active.role = Set(role.to_owned());
            active.updated_at = Set(chrono::Utc::now().into());
            return active.update(db).await.map_err(Into::into);
        }
        ActiveModel {
            team_id: Set(team.id),
            repository_id: Set(repository_id),
            role: Set(role.to_owned()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(Into::into)
    }

    /// Removes a team's grant. Repeated calls are a no-op.
    ///
    /// # Errors
    /// Returns a database error if the delete fails.
    pub async fn revoke(
        db: &DatabaseConnection,
        team_id: i64,
        repository_id: i64,
    ) -> ModelResult<()> {
        let found = Entity::find()
            .filter(Column::TeamId.eq(team_id))
            .filter(Column::RepositoryId.eq(repository_id))
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
