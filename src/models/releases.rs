//! Release metadata for git tags.
//!
//! The tag itself lives in the bare repository and is authoritative for the
//! commit it points at. This table only holds the human-facing parts: title,
//! notes, prerelease flag and the publisher. One release per tag per repository.

pub use super::_entities::releases::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

use crate::data::git;

pub type Releases = Entity;

/// Maximum length of a tag name, matching the controller's validation.
pub const TAG_NAME_MAX_LEN: usize = 100;

impl Model {
    /// Releases for a repository, newest first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_repository(
        db: &DatabaseConnection,
        repository_id: i64,
    ) -> ModelResult<Vec<Self>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .order_by_desc(Column::PublishedAt)
            .order_by_desc(Column::Id)
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// A release by its tag name.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_tag(
        db: &DatabaseConnection,
        repository_id: i64,
        tag_name: &str,
    ) -> ModelResult<Option<Self>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .filter(Column::TagName.eq(tag_name))
            .one(db)
            .await
            .map_err(Into::into)
    }

    /// The newest stable release, used for a "latest" shortcut.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_latest(
        db: &DatabaseConnection,
        repository_id: i64,
    ) -> ModelResult<Option<Self>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .filter(Column::IsPrerelease.eq(false))
            .order_by_desc(Column::PublishedAt)
            .order_by_desc(Column::Id)
            .one(db)
            .await
            .map_err(Into::into)
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Creates a release. A release always has a published timestamp; drafts
    /// are not modelled, so git and SQL never disagree about tag visibility.
    ///
    /// # Errors
    /// Returns a 400 for an invalid tag/title or a duplicate tag, or a
    /// database error.
    pub async fn create(
        db: &DatabaseConnection,
        repository_id: i64,
        author_id: i64,
        tag_name: &str,
        title: &str,
        body: Option<&str>,
        is_prerelease: bool,
    ) -> Result<Model> {
        validate_tag_name(tag_name)?;
        let title = title.trim();
        if title.is_empty() {
            return bad_request("release title must not be empty");
        }
        if Model::find_by_tag(db, repository_id, tag_name)
            .await?
            .is_some()
        {
            return bad_request("a release already exists for that tag");
        }
        ActiveModel {
            repository_id: Set(repository_id),
            tag_name: Set(tag_name.to_owned()),
            title: Set(title.to_owned()),
            body: Set(body.map(str::to_owned)),
            author_id: Set(author_id),
            is_prerelease: Set(is_prerelease),
            published_at: Set(Some(chrono::Utc::now().into())),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(Into::into)
    }

    /// Updates the human-facing fields; the tag name and target are immutable.
    ///
    /// # Errors
    /// Returns a 400 for a blank title, or a database error.
    pub async fn update_details(
        db: &DatabaseConnection,
        release: Model,
        title: &str,
        body: Option<&str>,
        is_prerelease: bool,
    ) -> Result<Model> {
        let title = title.trim();
        if title.is_empty() {
            return bad_request("release title must not be empty");
        }
        let mut active: ActiveModel = release.into();
        active.title = Set(title.to_owned());
        active.body = Set(body.map(str::to_owned));
        active.is_prerelease = Set(is_prerelease);
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(db).await.map_err(Into::into)
    }
}

/// Tag names become git ref path segments, so validate before reaching git.
///
/// # Errors
/// Returns a 400 when the name is not a safe ref component.
pub fn validate_tag_name(tag: &str) -> Result<()> {
    if !git::is_safe_tag(tag) || tag.len() > TAG_NAME_MAX_LEN {
        return bad_request("invalid tag name");
    }
    Ok(())
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
