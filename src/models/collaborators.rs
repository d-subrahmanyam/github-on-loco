//! Collaborator domain model: who may act on a repository besides its owner.
//!
//! Permissions are *not* decided here — [`super::repositories::Model::can`] is
//! the single rule. This module only stores, validates and finds role rows.

pub use super::_entities::collaborators::Column;
pub use super::_entities::collaborators::{ActiveModel, Entity, Model};
use super::repositories;
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type Collaborators = Entity;

/// Read-oriented domain logic.
impl Model {
    /// Whether this grant allows an action.
    #[must_use]
    pub fn allows(&self, action: repositories::RepoAction) -> bool {
        repositories::role_allows(&self.role, action)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Grants a role to a user who is not already a collaborator.
    ///
    /// # Errors
    /// Returns a 400 for an unknown role or a duplicate grant.
    pub async fn add(
        db: &DatabaseConnection,
        repository_id: i64,
        user_id: i64,
        role: &str,
    ) -> Result<Model> {
        if !repositories::is_collaborator_role(role) {
            return bad_request("invalid collaborator role");
        }
        if Entity::find_for(db, repository_id, user_id)
            .await?
            .is_some()
        {
            return bad_request("user is already a collaborator");
        }
        ActiveModel {
            repository_id: Set(repository_id),
            user_id: Set(user_id),
            role: Set(role.to_owned()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(Into::into)
    }

    /// Changes an existing grant's role.
    ///
    /// # Errors
    /// Returns a 400 for an unknown role.
    pub async fn set_role(mut self, db: &DatabaseConnection, role: &str) -> Result<Model> {
        if !repositories::is_collaborator_role(role) {
            return bad_request("invalid collaborator role");
        }
        self.role = Set(role.to_owned());
        self.update(db).await.map_err(Into::into)
    }
}

/// Custom finders.
impl Entity {
    /// All collaborators of a repository, with the most recently added first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_repository(
        db: &DatabaseConnection,
        repository_id: i64,
    ) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .order_by_desc(Column::Id)
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// The grant for one user in one repository, if any.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_for(
        db: &DatabaseConnection,
        repository_id: i64,
        user_id: i64,
    ) -> ModelResult<Option<Model>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await
            .map_err(Into::into)
    }

    /// Revokes a grant.
    ///
    /// # Errors
    /// Returns 404 when the user is not a collaborator.
    pub async fn remove(db: &DatabaseConnection, repository_id: i64, user_id: i64) -> Result<()> {
        let grant = Entity::find_for(db, repository_id, user_id).await?;
        let Some(grant) = grant else {
            return not_found();
        };
        ActiveModel {
            id: Set(grant.id),
            ..Default::default()
        }
        .delete(db)
        .await?;
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
