//! Webhook subscription management. Repository admins only: a webhook can
//! receive every event on the repository, including private activity.

#![allow(clippy::unused_async)]

use loco_rs::prelude::*;
use serde_json::json;

use crate::dtos::webhooks::{
    CreateWebhookParams, UpdateWebhookParams, WebhookDeliveryResponse, WebhookResponse,
};
use crate::models::{repositories, users, webhook_deliveries, webhooks, webhooks::EVENT_KINDS};
use crate::workers::deliver_webhook::{Worker as DeliverWebhookWorker, WorkerArgs};

const MASKED_SECRET: &str = "••••••••••••••••";

/// Resolves an optional JWT to a user id.
async fn viewer_id(ctx: &AppContext, viewer: Option<auth::JWT>) -> Result<Option<i64>> {
    match viewer {
        Some(jwt) => Ok(Some(
            users::Model::find_by_pid(&ctx.db, &jwt.claims.pid)
                .await?
                .id,
        )),
        None => Ok(None),
    }
}

/// Resolves a repository and requires admin access, answering 404 on denial.
async fn find_admin_repo(
    ctx: &AppContext,
    owner: &str,
    repo: &str,
    viewer: Option<i64>,
) -> Result<repositories::Model> {
    let row = repositories::Model::find_by_owner_and_name(&ctx.db, owner, repo).await?;
    let Some(user_id) = viewer else {
        return not_found();
    };
    if !row.can_admin(&ctx.db, user_id).await? {
        return not_found();
    }
    Ok(row)
}

async fn creator_name(ctx: &AppContext, user_id: i64) -> Result<String> {
    Ok(users::Entity::find_by_id(user_id)
        .one(&ctx.db)
        .await?
        .map(|user| user.username)
        .unwrap_or_else(|| "unknown".to_owned()))
}

fn to_response(model: webhooks::Model, created_by: String, reveal_secret: bool) -> WebhookResponse {
    WebhookResponse {
        id: model.id,
        url: model.url,
        events: model.events,
        is_active: model.is_active,
        created_by,
        secret: if reveal_secret {
            model.secret
        } else {
            MASKED_SECRET.to_owned()
        },
    }
}

fn delivery_response(model: webhook_deliveries::Model) -> WebhookDeliveryResponse {
    WebhookDeliveryResponse {
        id: model.id,
        event: model.event,
        status: model.status,
        attempts: model.attempts,
        last_error: model.last_error,
        created_at: model.created_at.to_rfc3339(),
        delivered_at: model.delivered_at.map(|date| date.to_rfc3339()),
    }
}

/// `GET /api/webhook-events` — the event kinds a subscription may use.
#[debug_handler]
pub async fn event_kinds() -> Result<Response> {
    format::json(EVENT_KINDS)
}

/// `GET /api/repos/{owner}/{repo}/webhooks`.
#[debug_handler]
pub async fn list(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_admin_repo(&ctx, &owner, &repo, viewer).await?;
    let records = webhooks::Model::find_by_repository(&ctx.db, row.id).await?;
    let mut response = Vec::with_capacity(records.len());
    for hook in records {
        let creator = creator_name(&ctx, hook.created_by).await?;
        response.push(to_response(hook, creator, false));
    }
    format::json(response)
}

/// `POST /api/repos/{owner}/{repo}/webhooks`.
#[debug_handler]
pub async fn create(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateWebhookParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_admin_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    let hook = webhooks::ActiveModel::create(&ctx.db, row.id, user.id, &params.url, &params.events)
        .await?;
    format::json(to_response(hook, user.username, true))
}

/// `GET /api/repos/{owner}/{repo}/webhooks/{id}`.
#[debug_handler]
pub async fn show(
    Path((owner, repo, id)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_admin_repo(&ctx, &owner, &repo, viewer).await?;
    let hook = webhooks::Model::find_in_repository(&ctx.db, row.id, id).await?;
    let creator = creator_name(&ctx, hook.created_by).await?;
    format::json(to_response(hook, creator, false))
}

/// `PATCH /api/repos/{owner}/{repo}/webhooks/{id}`.
#[debug_handler]
pub async fn update(
    auth: auth::JWT,
    Path((owner, repo, id)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateWebhookParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_admin_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    let hook = webhooks::Model::find_in_repository(&ctx.db, row.id, id).await?;
    let hook =
        webhooks::ActiveModel::update_subscription(&ctx.db, hook, &params.events, params.is_active)
            .await?;
    format::json(to_response(hook, user.username.clone(), false))
}

/// `POST /api/repos/{owner}/{repo}/webhooks/{id}/rotate`.
#[debug_handler]
pub async fn rotate_secret(
    auth: auth::JWT,
    Path((owner, repo, id)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_admin_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    let hook = webhooks::Model::find_in_repository(&ctx.db, row.id, id).await?;
    let hook = webhooks::ActiveModel::rotate_secret(&ctx.db, hook).await?;
    format::json(json!({ "secret": hook.secret }))
}

/// `DELETE /api/repos/{owner}/{repo}/webhooks/{id}`.
#[debug_handler]
pub async fn remove(
    auth: auth::JWT,
    Path((owner, repo, id)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_admin_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    let hook = webhooks::Model::find_in_repository(&ctx.db, row.id, id).await?;
    let active: webhooks::ActiveModel = hook.into();
    active.delete(&ctx.db).await?;
    format::empty_json()
}

/// `GET /api/repos/{owner}/{repo}/webhooks/{id}/deliveries`.
#[debug_handler]
pub async fn deliveries(
    Path((owner, repo, id)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_admin_repo(&ctx, &owner, &repo, viewer).await?;
    let hook = webhooks::Model::find_in_repository(&ctx.db, row.id, id).await?;
    let records = webhook_deliveries::Model::find_by_webhook(&ctx.db, hook.id, 50).await?;
    format::json(
        records
            .into_iter()
            .map(delivery_response)
            .collect::<Vec<_>>(),
    )
}

/// `POST /api/repos/{owner}/{repo}/webhooks/{id}/deliveries/{delivery_id}/retry`.
#[debug_handler]
pub async fn retry_delivery(
    auth: auth::JWT,
    Path((owner, repo, id, delivery_id)): Path<(String, String, i64, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_admin_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    let hook = webhooks::Model::find_in_repository(&ctx.db, row.id, id).await?;
    let delivery = webhook_deliveries::Entity::find_by_id(delivery_id)
        .one(&ctx.db)
        .await?
        .filter(|row| row.webhook_id == hook.id)
        .ok_or(Error::NotFound)?;
    DeliverWebhookWorker::perform_later(
        &ctx,
        WorkerArgs {
            delivery_id: delivery.id,
        },
    )
    .await?;
    format::empty_json()
}

/// Repository-scoped routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/repos/{owner}/{repo}")
        .add("/webhooks", get(list))
        .add("/webhooks", post(create))
        .add("/webhooks/{id}", get(show))
        .add("/webhooks/{id}", patch(update))
        .add("/webhooks/{id}", delete(remove))
        .add("/webhooks/{id}/rotate", post(rotate_secret))
        .add("/webhooks/{id}/deliveries", get(deliveries))
        .add(
            "/webhooks/{id}/deliveries/{delivery_id}/retry",
            post(retry_delivery),
        )
}

/// Global route for the enum of subscribable event kinds.
pub fn event_routes() -> Routes {
    Routes::new().add("/api/webhook-events", get(event_kinds))
}
