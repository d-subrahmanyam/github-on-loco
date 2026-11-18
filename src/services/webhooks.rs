//! Turns domain events into queued webhook deliveries.
//!
//! Kept separate from `models::events` so the activity log stays a pure append
//! and this module can reach the queue without making every caller depend on it.

use hmac::{Hmac, KeyInit, Mac};
use loco_rs::prelude::*;
use sea_orm::EntityTrait;
use serde_json::{json, Value};
use sha2::Sha256;

use crate::models::{organizations, repositories, users, webhook_deliveries, webhooks};
use crate::workers::deliver_webhook::{Worker as DeliverWebhookWorker, WorkerArgs};

type HmacSha256 = Hmac<Sha256>;

/// Signs a raw payload the way GitHub does: `sha256=<hex HMAC>`.
///
/// # Errors
/// Returns an error only if the secret cannot initialize the HMAC key.
pub fn sign(secret: &str, body: &str) -> Result<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|err| Error::string(&format!("invalid webhook secret: {err}")))?;
    mac.update(body.as_bytes());
    Ok(format!(
        "sha256={}",
        hex::encode(mac.finalize().into_bytes())
    ))
}

/// Resolves the owner handle for a repository, which may be an organization.
async fn owner_handle(
    db: &sea_orm::DatabaseConnection,
    repo: &repositories::Model,
) -> Result<String> {
    if let Some(organization_id) = repo.organization_id {
        Ok(organizations::Entity::find_by_id(organization_id)
            .one(db)
            .await?
            .map(|org| org.handle)
            .unwrap_or_else(|| "unknown".to_owned()))
    } else {
        Ok(users::Entity::find_by_id(repo.user_id)
            .one(db)
            .await?
            .map(|user| user.username)
            .unwrap_or_else(|| "unknown".to_owned()))
    }
}

/// Enqueues one delivery per subscribed, active webhook.
///
/// Dispatch failures are returned to the caller, which is expected to log and
/// swallow them: a broken webhook must never fail the domain action that
/// triggered it.
///
/// # Errors
/// Returns a database or queue error.
pub async fn dispatch(
    ctx: &AppContext,
    repository_id: i64,
    event: &str,
    actor_id: i64,
    extra: Option<&Value>,
) -> Result<()> {
    let hooks = webhooks::Model::find_matching(&ctx.db, repository_id, event).await?;
    if hooks.is_empty() {
        return Ok(());
    }

    let Some(repo) = repositories::Entity::find_by_id(repository_id)
        .one(&ctx.db)
        .await?
    else {
        return Ok(());
    };
    let owner = owner_handle(&ctx.db, &repo).await?;
    let sender = users::Entity::find_by_id(actor_id)
        .one(&ctx.db)
        .await?
        .map(|user| user.username)
        .unwrap_or_else(|| "unknown".to_owned());

    let payload = json!({
        "event": event,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "repository": {
            "id": repo.id,
            "owner": owner,
            "name": repo.display_name()?,
            "private": repo.is_private(),
        },
        "sender": { "id": actor_id, "username": sender },
        "payload": extra.cloned().unwrap_or(Value::Null),
    });

    for hook in hooks {
        let delivery =
            webhook_deliveries::ActiveModel::create(&ctx.db, hook.id, event, &payload).await?;
        DeliverWebhookWorker::perform_later(
            ctx,
            WorkerArgs {
                delivery_id: delivery.id,
            },
        )
        .await?;
    }
    Ok(())
}
