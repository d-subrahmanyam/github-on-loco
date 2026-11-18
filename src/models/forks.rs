//! Fork relationships: which repository a fork was copied from.
//!
//! The repository row owns the fork's metadata and the bare git directory; this
//! table only records the source so a fork network can be rendered and a fork
//! can point back to its upstream.

pub use super::_entities::forks::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

use super::{repositories, users};

pub type Forks = Entity;

/// Read-oriented domain logic.
impl Entity {
    /// All forks of a source repository, newest first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_source(
        db: &DatabaseConnection,
        source_repository_id: i64,
    ) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::SourceRepositoryId.eq(source_repository_id))
            .order_by_desc(Column::Id)
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// Number of forks of a source repository.
    ///
    /// # Errors
    /// Returns a database error if the count fails.
    pub async fn count_for_source(
        db: &DatabaseConnection,
        source_repository_id: i64,
    ) -> Result<i64> {
        Ok(Entity::find()
            .filter(Column::SourceRepositoryId.eq(source_repository_id))
            .count(db)
            .await? as i64)
    }

    /// The fork row for a repository, if that repository is a fork.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_for_repository(
        db: &DatabaseConnection,
        repository_id: i64,
    ) -> ModelResult<Option<Model>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .one(db)
            .await
            .map_err(Into::into)
    }

    /// `(owner handle, name)` of the source, for rendering "forked from …".
    ///
    /// # Errors
    /// Returns a database error if any lookup fails.
    pub async fn source_label(
        db: &DatabaseConnection,
        repository_id: i64,
    ) -> Result<Option<(String, String)>> {
        let Some(fork) = Self::find_for_repository(db, repository_id).await? else {
            return Ok(None);
        };
        let Some(source) = repositories::Entity::find_by_id(fork.source_repository_id)
            .one(db)
            .await?
        else {
            return Ok(None);
        };
        let owner = users::Entity::find_by_id(source.user_id)
            .one(db)
            .await?
            .map(|user| user.owner_handle().to_owned())
            .unwrap_or_else(|| "unknown".to_owned());
        Ok(Some((owner, source.display_name()?.to_owned())))
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Records a newly created fork. Callers create the repository and its git
    /// directory first, then link them here.
    ///
    /// # Errors
    /// Returns a database error if the insert fails.
    pub async fn create(
        db: &DatabaseConnection,
        source_repository_id: i64,
        repository_id: i64,
    ) -> Result<Model> {
        ActiveModel {
            source_repository_id: Set(source_repository_id),
            repository_id: Set(repository_id),
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
