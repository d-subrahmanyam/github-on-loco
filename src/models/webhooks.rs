//! Outbound webhook subscriptions for a repository.
//!
//! A webhook stores the destination URL, the event kinds it subscribes to and
//! an HMAC secret. Deliveries are separate rows so failures can be retried and
//! the SPA can show a history without re-reading git.

pub use super::_entities::webhooks::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::entity::prelude::*;

use super::events::kind as event;

pub type Webhooks = Entity;

/// Subscribes to every event kind.
pub const ALL_EVENTS: &str = "*";

/// The event kinds a webhook may subscribe to. Kept explicit so a typo in the
/// SPA or API cannot silently create a subscription that never fires.
pub const EVENT_KINDS: &[&str] = &[
    event::STAR,
    event::UNSTAR,
    event::WATCH,
    event::UNWATCH,
    event::FORK,
    event::ISSUE_OPENED,
    event::ISSUE_CLOSED,
    event::ISSUE_REOPENED,
    event::PR_OPENED,
    event::PR_MERGED,
    event::PR_APPROVED,
    event::PR_CHANGES_REQUESTED,
    event::PR_REVIEWED,
    event::COLLABORATOR_ADDED,
    event::COLLABORATOR_REMOVED,
    event::PUSH,
    event::RELEASE_PUBLISHED,
    event::RELEASE_UPDATED,
    event::RELEASE_DELETED,
    event::TAG_DELETED,
];

impl Model {
    /// Whether this webhook is subscribed to `event_name`.
    #[must_use]
    pub fn subscribes(&self, event_name: &str) -> bool {
        self.events
            .split(',')
            .map(str::trim)
            .any(|kind| kind == ALL_EVENTS || kind == event_name)
    }

    /// All webhooks for a repository, oldest first.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_by_repository(
        db: &DatabaseConnection,
        repository_id: i64,
    ) -> ModelResult<Vec<Self>> {
        Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .order_by_asc(Column::Id)
            .all(db)
            .await
            .map_err(Into::into)
    }

    /// One webhook scoped to a repository, so another repo cannot reach it.
    ///
    /// # Errors
    /// Returns `EntityNotFound` when the id is absent or belongs elsewhere.
    pub async fn find_in_repository(
        db: &DatabaseConnection,
        repository_id: i64,
        id: i64,
    ) -> ModelResult<Self> {
        Entity::find()
            .filter(Column::Id.eq(id))
            .filter(Column::RepositoryId.eq(repository_id))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)
    }

    /// Active webhooks subscribed to `event_name`.
    ///
    /// # Errors
    /// Returns a database error if the query fails.
    pub async fn find_matching(
        db: &DatabaseConnection,
        repository_id: i64,
        event_name: &str,
    ) -> Result<Vec<Self>> {
        let rows = Self::find_by_repository(db, repository_id).await?;
        Ok(rows
            .into_iter()
            .filter(|row| row.is_active && row.subscribes(event_name))
            .collect())
    }
}

/// Write-oriented domain logic.
impl ActiveModel {
    /// Creates a webhook with a freshly generated signing secret.
    ///
    /// # Errors
    /// Returns a 400 for an invalid URL/events list or a duplicate URL, or a
    /// database error.
    pub async fn create(
        db: &DatabaseConnection,
        repository_id: i64,
        created_by: i64,
        url: &str,
        events: &str,
    ) -> Result<Model> {
        validate_url(url)?;
        let events = normalize_events(events)?;
        if Entity::find()
            .filter(Column::RepositoryId.eq(repository_id))
            .filter(Column::Url.eq(url))
            .one(db)
            .await?
            .is_some()
        {
            return bad_request("a webhook with that URL already exists");
        }
        let secret = new_secret();
        ActiveModel {
            repository_id: Set(repository_id),
            url: Set(url.to_owned()),
            events: Set(events),
            secret: Set(secret),
            is_active: Set(true),
            created_by: Set(created_by),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(Into::into)
    }

    /// Updates the subscription and active flag; the secret is stable.
    ///
    /// # Errors
    /// Returns a 400 for an invalid events list, or a database error.
    pub async fn update_subscription(
        db: &DatabaseConnection,
        webhook: Model,
        events: &str,
        is_active: bool,
    ) -> Result<Model> {
        let events = normalize_events(events)?;
        let mut active: ActiveModel = webhook.into();
        active.events = Set(events);
        active.is_active = Set(is_active);
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(db).await.map_err(Into::into)
    }

    /// Rotates the signing secret, invalidating the old one immediately.
    ///
    /// # Errors
    /// Returns a database error if the update fails.
    pub async fn rotate_secret(db: &DatabaseConnection, webhook: Model) -> Result<Model> {
        let mut active: ActiveModel = webhook.into();
        active.secret = Set(new_secret());
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(db).await.map_err(Into::into)
    }
}

/// Generates a new `whsec_…` signing secret.
#[must_use]
pub fn new_secret() -> String {
    format!("whsec_{}", uuid::Uuid::new_v4().simple())
}

/// Validates and normalises a comma-separated events list.
///
/// # Errors
/// Returns a 400 for an empty list or an unknown event kind.
pub fn normalize_events(events: &str) -> Result<String> {
    let parts: Vec<&str> = events
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return bad_request("at least one event is required");
    }
    for kind in &parts {
        if *kind != ALL_EVENTS && !EVENT_KINDS.contains(kind) {
            return bad_request(format!("unknown webhook event: {kind}"));
        }
    }
    if parts.contains(&ALL_EVENTS) {
        return Ok(ALL_EVENTS.to_owned());
    }
    Ok(parts.join(","))
}

/// Webhook endpoints must be absolute http(s) URLs.
///
/// # Errors
/// Returns a 400 for an empty or non-http(s) URL.
pub fn validate_url(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) || trimmed.len() <= 8 {
        return bad_request("webhook URL must be an absolute http(s) URL");
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
