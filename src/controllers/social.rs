//! Stars, watches and the repository activity feed.

#![allow(clippy::unused_async)]

use loco_rs::prelude::*;
use serde::Deserialize;
use serde_json::Value;

use crate::dtos::social::{EventResponse, RepoStatsResponse};
use crate::models::{events, repositories, stars, users, watches};

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

async fn find_repo(
    ctx: &AppContext,
    owner: &str,
    repo: &str,
    viewer: Option<i64>,
) -> Result<repositories::Model> {
    let row = repositories::Model::find_by_owner_and_name(&ctx.db, owner, repo).await?;
    if !row.can_read(&ctx.db, viewer).await? {
        return not_found();
    }
    Ok(row)
}

/// Builds the counts and the viewer's state in one place.
async fn stats(ctx: &AppContext, repository_id: i64, user_id: i64) -> Result<RepoStatsResponse> {
    Ok(RepoStatsResponse {
        star_count: stars::Entity::count_for_repository(&ctx.db, repository_id).await?,
        watch_count: watches::Entity::count_for_repository(&ctx.db, repository_id).await?,
        starred: stars::Entity::is_starred(&ctx.db, repository_id, user_id).await?,
        watching: watches::Entity::is_watching(&ctx.db, repository_id, user_id).await?,
    })
}

/// Turns a small JSON payload into a sentence for the feed.
fn event_text(event: &events::Model) -> String {
    let field = |key: &str| {
        event
            .payload
            .as_ref()
            .and_then(|value| value.get(key))
            .map(|value| match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default()
    };
    match event.kind.as_str() {
        events::kind::STAR => "starred this repository".to_owned(),
        events::kind::UNSTAR => "unstarred this repository".to_owned(),
        events::kind::WATCH => "started watching this repository".to_owned(),
        events::kind::UNWATCH => "stopped watching this repository".to_owned(),
        events::kind::ISSUE_OPENED => format!("opened issue #{}", field("number")),
        events::kind::ISSUE_CLOSED => format!("closed issue #{}", field("number")),
        events::kind::ISSUE_REOPENED => format!("reopened issue #{}", field("number")),
        events::kind::PR_OPENED => format!("opened pull request #{}", field("number")),
        events::kind::PR_MERGED => format!("merged pull request #{}", field("number")),
        events::kind::PR_APPROVED => format!("approved pull request #{}", field("number")),
        events::kind::PR_CHANGES_REQUESTED => {
            format!("requested changes on pull request #{}", field("number"))
        }
        events::kind::PR_REVIEWED => format!("reviewed pull request #{}", field("number")),
        events::kind::COLLABORATOR_ADDED => {
            format!("added @{} as {}", field("target"), field("role"))
        }
        events::kind::COLLABORATOR_REMOVED => {
            format!("removed @{} as a collaborator", field("target"))
        }
        events::kind::PUSH => format!("pushed to {}", field("branch")),
        other => other.to_owned(),
    }
}

/// `POST /api/repos/{owner}/{repo}/star`.
#[debug_handler]
pub async fn star(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if stars::ActiveModel::star(&ctx.db, row.id, user.id).await? {
        events::Entity::record(&ctx.db, row.id, user.id, events::kind::STAR, None).await?;
    }
    format::json(stats(&ctx, row.id, user.id).await?)
}

/// `DELETE /api/repos/{owner}/{repo}/star`.
#[debug_handler]
pub async fn unstar(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if stars::Entity::unstar(&ctx.db, row.id, user.id).await? {
        events::Entity::record(&ctx.db, row.id, user.id, events::kind::UNSTAR, None).await?;
    }
    format::json(stats(&ctx, row.id, user.id).await?)
}

/// `POST /api/repos/{owner}/{repo}/watch`.
#[debug_handler]
pub async fn watch(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if watches::ActiveModel::watch(&ctx.db, row.id, user.id).await? {
        events::Entity::record(&ctx.db, row.id, user.id, events::kind::WATCH, None).await?;
    }
    format::json(stats(&ctx, row.id, user.id).await?)
}

/// `DELETE /api/repos/{owner}/{repo}/watch`.
#[debug_handler]
pub async fn unwatch(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if watches::Entity::unwatch(&ctx.db, row.id, user.id).await? {
        events::Entity::record(&ctx.db, row.id, user.id, events::kind::UNWATCH, None).await?;
    }
    format::json(stats(&ctx, row.id, user.id).await?)
}

#[derive(Debug, Deserialize)]
pub struct EventListQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

/// `GET /api/repos/{owner}/{repo}/events?limit=20`.
#[debug_handler]
pub async fn events(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Query(query): Query<EventListQuery>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let records = events::Entity::find_by_repository(&ctx.db, row.id, query.limit).await?;
    let mut response = Vec::with_capacity(records.len());
    for event in records {
        let actor = users::Entity::find_by_id(event.user_id)
            .one(&ctx.db)
            .await?
            .map(|user| user.username)
            .unwrap_or_else(|| "unknown".to_owned());
        response.push(EventResponse {
            id: event.id,
            actor,
            kind: event.kind.clone(),
            text: event_text(&event),
            created_at: event.created_at.to_rfc3339(),
        });
    }
    format::json(response)
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/repos/{owner}/{repo}")
        .add("/star", post(star))
        .add("/star", delete(unstar))
        .add("/watch", post(watch))
        .add("/watch", delete(unwatch))
        .add("/events", get(events))
}
