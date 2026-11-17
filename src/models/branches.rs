//! Cache of branch heads for a repository.
//!
//! Git is authoritative; these rows exist so the branch list can be answered
//! without spawning a process. A push invalidates them (see `git_http`), and a
//! cache miss is refilled from git on the next read.

use super::_entities::branches::Column;
pub use super::_entities::branches::{ActiveModel, Entity, Model};
use crate::dtos::repositories::Branch;
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type Branches = Entity;

/// Custom finders and cache maintenance.
impl Entity {
    /// Cached branch heads for a repository, or an empty list on a miss/error.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_repository(
        db: &DatabaseConnection,
        repository_id: i64,
    ) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// Replaces the whole cache for a repository in one transaction.
    ///
    /// # Errors
    /// Returns a database error if any statement fails.
    pub async fn replace_for_repository(
        db: &DatabaseConnection,
        repository_id: i64,
        branches: &[Branch],
    ) -> Result<()> {
        let txn = db.begin().await?;
        Entity::delete_many()
            .filter(Column::RepositoryId.eq(repository_id))
            .exec(&txn)
            .await?;
        for branch in branches {
            ActiveModel {
                repository_id: Set(repository_id),
                name: Set(branch.name.clone()),
                sha: Set(branch.sha.clone()),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
        txn.commit().await?;
        Ok(())
    }

    /// Drops every cached branch for a repository. Called after a push.
    ///
    /// # Errors
    /// Returns a database error if the delete fails.
    pub async fn clear_for_repository(db: &DatabaseConnection, repository_id: i64) -> Result<()> {
        Entity::delete_many()
            .filter(Column::RepositoryId.eq(repository_id))
            .exec(db)
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
