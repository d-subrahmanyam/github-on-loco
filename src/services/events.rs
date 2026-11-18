//! Records an activity event and fans it out to subscribed webhooks.
//!
//! All controllers use this instead of calling `events::Entity::record`
//! directly. Webhook dispatch is best-effort: a broken subscription is logged
//! but never fails the action that produced the event.

use loco_rs::prelude::*;
use serde_json::Value;

use crate::models::events;
use crate::services::{notifications, webhooks};

/// Appends the event, then queues matching webhook deliveries.
///
/// # Errors
/// Returns an error only if the activity event itself cannot be recorded.
pub async fn record(
    ctx: &AppContext,
    repository_id: i64,
    actor_id: i64,
    kind: &str,
    payload: Option<Value>,
) -> Result<()> {
    events::Entity::record(&ctx.db, repository_id, actor_id, kind, payload.clone()).await?;
    if let Err(err) = webhooks::dispatch(ctx, repository_id, kind, actor_id, payload.as_ref()).await
    {
        tracing::warn!(%err, event = kind, "webhook dispatch failed");
    }
    if let Err(err) =
        notifications::dispatch(ctx, repository_id, kind, actor_id, payload.as_ref()).await
    {
        tracing::warn!(%err, event = kind, "notification dispatch failed");
    }
    Ok(())
}
