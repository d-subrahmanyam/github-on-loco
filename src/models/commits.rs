//! Cache of recent commits for a repository's default branch.
//!
//! Only the default branch's history is cached — an explicit `?ref=` request
//! always reads git directly, because this table has no ref column. A push
//! invalidates the cache; a miss is refilled from git on the next read.

use super::_entities::commits::Column;
pub use super::_entities::commits::{ActiveModel, Entity, Model};
use crate::dtos::repositories::Commit;
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type Commits = Entity;

/// Custom finders and cache maintenance.
impl Entity {
    /// Cached commits for a repository.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_repository(
        db: &DatabaseConnection,
        repository_id: i64,
    ) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .order_by_desc(Column::CommittedAt)
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// Replaces the whole cache for a repository in one transaction.
    ///
    /// # Errors
    /// Returns a database error for a malformed timestamp or a failed query.
    pub async fn replace_for_repository(
        db: &DatabaseConnection,
        repository_id: i64,
        commits: &[Commit],
    ) -> Result<()> {
        let txn = db.begin().await?;
        Entity::delete_many()
            .filter(Column::RepositoryId.eq(repository_id))
            .exec(&txn)
            .await?;
        for commit in commits {
            let committed_at = chrono::DateTime::parse_from_rfc3339(&commit.committed_at)
                .map_err(|e| Error::string(&format!("invalid commit timestamp: {e}")))?;
            ActiveModel {
                repository_id: Set(repository_id),
                sha: Set(commit.sha.clone()),
                message: Set(commit.message.clone()),
                author: Set(commit.author.clone()),
                committed_at: Set(committed_at),
                parents: Set(None),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
        txn.commit().await?;
        Ok(())
    }

    /// Drops every cached commit for a repository. Called after a push.
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
