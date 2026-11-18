//! Watches: a user subscribing to a repository's activity.

pub use super::_entities::watches::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type Watches = Entity;

impl Entity {
    /// Whether a user is watching a repository.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn is_watching(
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

    /// Ids of every user watching a repository, oldest watch first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn watcher_ids(db: &DatabaseConnection, repository_id: i64) -> Result<Vec<i64>> {
        Ok(Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .order_by_asc(Column::Id)
            .all(db)
            .await?
            .into_iter()
            .map(|row| row.user_id)
            .collect())
    }

    /// Number of watchers on a repository.
    ///
    /// # Errors
    /// Returns a database error if the count fails.
    pub async fn count_for_repository(db: &DatabaseConnection, repository_id: i64) -> Result<i64> {
        Ok(Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .count(db)
            .await? as i64)
    }

    /// Removes a watch. Repeated calls are a no-op.
    ///
    /// # Errors
    /// Returns a database error if the delete fails.
    pub async fn unwatch(
        db: &DatabaseConnection,
        repository_id: i64,
        user_id: i64,
    ) -> Result<bool> {
        let watch = Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await?;
        let Some(watch) = watch else {
            return Ok(false);
        };
        let active: ActiveModel = watch.into();
        active.delete(db).await?;
        Ok(true)
    }
}

impl ActiveModel {
    /// Watches a repository. Repeated calls are a no-op. Returns `true` when a
    /// new watch was created.
    ///
    /// # Errors
    /// Returns a database error if the insert fails.
    pub async fn watch(db: &DatabaseConnection, repository_id: i64, user_id: i64) -> Result<bool> {
        if Entity::is_watching(db, repository_id, user_id).await? {
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
