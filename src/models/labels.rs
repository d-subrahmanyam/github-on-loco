//! Repository-scoped issue labels.

pub use super::_entities::labels::Column;
pub use super::_entities::labels::{ActiveModel, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

pub type Labels = Entity;

/// Custom finders.
impl Entity {
    /// Every label defined for a repository.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_repository(
        db: &DatabaseConnection,
        repository_id: i64,
    ) -> ModelResult<Vec<Model>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .order_by_asc(Column::Name)
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// Finds a label by name within a repository.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_name(
        db: &DatabaseConnection,
        repository_id: i64,
        name: &str,
    ) -> ModelResult<Option<Model>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .filter(Column::Name.eq(name))
            .one(db)
            .await
            .map_err(Into::into)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Creates a label after validating the name and hex colour.
    ///
    /// # Errors
    /// Returns a 400 for invalid input, or a database error on insert.
    pub async fn create_for_repository(
        db: &DatabaseConnection,
        repository_id: i64,
        name: &str,
        color: &str,
    ) -> Result<Model> {
        let name = name.trim();
        let color = color.trim().trim_start_matches('#');
        if name.is_empty() || name.len() > 50 {
            return bad_request("label name must be 1-50 characters");
        }
        if color.len() != 6 || !color.chars().all(|c| c.is_ascii_hexdigit()) {
            return bad_request("label color must be a 6-digit hex code");
        }
        ActiveModel {
            repository_id: Set(repository_id),
            name: Set(name.to_owned()),
            color: Set(color.to_owned()),
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
