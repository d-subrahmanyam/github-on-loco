//! Stars: a user bookmarking a repository. One row per (user, repository).

pub use super::_entities::stars::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type Stars = Entity;

impl Entity {
    /// Whether a user has starred a repository.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn is_starred(
        db: &DatabaseConnection,
        repository_id: i64,
        user_id: i64,
    ) -> Result<bool> {
        Ok(Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await?
            .is_some())
    }

    /// Number of stars on a repository.
    ///
    /// # Errors
    /// Returns a database error if the count fails.
    pub async fn count_for_repository(db: &DatabaseConnection, repository_id: i64) -> Result<i64> {
        Ok(Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .count(db)
            .await? as i64)
    }
}

impl ActiveModel {
    /// Stars a repository. Repeated calls are a no-op. Returns `true` when a
    /// new star was created, so callers can record an event only on a change.
    ///
    /// # Errors
    /// Returns a database error if the insert fails.
    pub async fn star(db: &DatabaseConnection, repository_id: i64, user_id: i64) -> Result<bool> {
        if Entity::is_starred(db, repository_id, user_id).await? {
            return Ok(false);
        }
        ActiveModel {
            repository_id: Set(repository_id),
            user_id: Set(user_id),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(true)
    }
}

impl Entity {
    /// Removes a star. Repeated calls are a no-op. Returns `true` when a star
    /// was actually removed.
    ///
    /// # Errors
    /// Returns a database error if the delete fails.
    pub async fn unstar(db: &DatabaseConnection, repository_id: i64, user_id: i64) -> Result<bool> {
        let star = Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await?;
        let Some(star) = star else {
            return Ok(false);
        };
        let active: ActiveModel = star.into();
        active.delete(db).await?;
        Ok(true)
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
