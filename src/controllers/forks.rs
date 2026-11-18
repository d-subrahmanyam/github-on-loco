//! Fork endpoints. A fork is a new repository owned by the caller, with the
//! source's commits copied by `git clone --bare`. Metadata and the fork edge
//! live in Postgres; git remains authoritative for the contents.

#![allow(clippy::unused_async)]

use loco_rs::prelude::*;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;
use std::path::Path as FsPath;
use tokio::process::Command;

use crate::data::settings::AppSettings;
use crate::dtos::forks::{CreateForkParams, ForkParentResponse, ForkResponse};
use crate::dtos::repositories::RepoResponse;
use crate::models::{events, forks, repositories, users};

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

/// Resolves a repository and enforces read visibility with a 404.
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

/// Copies a bare repository. The destination must not already exist.
async fn clone_bare_repo(source: &FsPath, dest: &FsPath) -> Result<()> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::string(&format!("failed to create fork directory: {e}")))?;
    }
    let status = Command::new("git")
        .arg("clone")
        .arg("--bare")
        .arg("--quiet")
        .arg(source)
        .arg(dest)
        .status()
        .await
        .map_err(|e| Error::string(&format!("failed to execute git clone: {e}")))?;
    if !status.success() {
        return Err(Error::string("git clone --bare failed"));
    }
    Ok(())
}

/// Picks a name for the fork: the requested name, or a free `-fork-N` variant
/// when the caller already owns a repository with that name.
async fn available_name(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    requested: &str,
) -> Result<String> {
    repositories::validate_name(requested)?;
    let owned = repositories::Model::find_by_user(db, user_id).await?;
    let taken: HashSet<&str> = owned.iter().filter_map(|row| row.name.as_deref()).collect();
    if !taken.contains(requested) {
        return Ok(requested.to_owned());
    }
    for number in 1..=100 {
        let candidate = format!("{requested}-fork-{number}");
        if candidate.len() <= repositories::NAME_MAX_LEN && !taken.contains(candidate.as_str()) {
            return Ok(candidate);
        }
    }
    bad_request("could not find an available fork name")
}

/// `POST /api/repos/{owner}/{repo}/fork`.
#[debug_handler]
pub async fn create(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateForkParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let source = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if source.is_owned_by(user.id) {
        return bad_request("you cannot fork your own repository");
    }
    let requested = params
        .name
        .unwrap_or_else(|| source.display_name().unwrap_or("repo").to_owned());
    let name = available_name(&ctx.db, user.id, &requested).await?;

    let fork_repo = repositories::ActiveModel::create_for_user(
        &ctx.db,
        user.id,
        &name,
        source.description.as_deref(),
        source.is_private(),
    )
    .await?;

    let settings = AppSettings::from_context(&ctx)?;
    let source_dir =
        repositories::repo_dir(&settings.repo_base_dir, &owner, source.display_name()?)?;
    let dest_dir = repositories::repo_dir(&settings.repo_base_dir, user.owner_handle(), &name)?;

    // Two stores again: roll the row back if the git side fails, and remove the
    // directory if recording the fork edge fails.
    if let Err(err) = clone_bare_repo(&source_dir, &dest_dir).await {
        let active: repositories::ActiveModel = fork_repo.clone().into();
        active.delete(&ctx.db).await?;
        return Err(err);
    }
    if let Err(err) = forks::ActiveModel::create(&ctx.db, source.id, fork_repo.id).await {
        let active: repositories::ActiveModel = fork_repo.clone().into();
        active.delete(&ctx.db).await?;
        let _ = tokio::fs::remove_dir_all(&dest_dir).await;
        return Err(err);
    }

    crate::services::events::record(
        &ctx,
        source.id,
        user.id,
        events::kind::FORK,
        Some(json!({ "owner": user.owner_handle(), "name": name })),
    )
    .await?;

    format::json(RepoResponse {
        id: fork_repo.id,
        owner: user.owner_handle().to_owned(),
        name: name.clone(),
        description: fork_repo.description.clone(),
        is_private: fork_repo.is_private(),
        clone_url: format!(
            "{}{}",
            ctx.config.server.full_url(),
            fork_repo.clone_path(user.owner_handle())?
        ),
        viewer_role: repositories::ROLE_OWNER.to_owned(),
        star_count: 0,
        watch_count: 0,
        starred: false,
        watching: false,
        fork_count: 0,
        parent: Some(ForkParentResponse {
            owner: owner.clone(),
            name: source.display_name()?.to_owned(),
        }),
    })
}

#[derive(Debug, Deserialize)]
pub struct ForkListQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

/// `GET /api/repos/{owner}/{repo}/forks?limit=50`.
#[debug_handler]
pub async fn list(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Query(query): Query<ForkListQuery>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let records = forks::Entity::find_by_source(&ctx.db, row.id).await?;
    let mut response = Vec::with_capacity(records.len().min(query.limit));
    for record in records {
        if response.len() >= query.limit {
            break;
        }
        let Some(fork_repo) = repositories::Entity::find_by_id(record.repository_id)
            .one(&ctx.db)
            .await?
        else {
            continue;
        };
        let owner_handle = users::Entity::find_by_id(fork_repo.user_id)
            .one(&ctx.db)
            .await?
            .map(|user| user.owner_handle().to_owned())
            .unwrap_or_else(|| "unknown".to_owned());
        response.push(ForkResponse {
            id: record.id,
            repository_id: fork_repo.id,
            owner: owner_handle,
            name: fork_repo.display_name()?.to_owned(),
            created_at: record.created_at.to_rfc3339(),
        });
    }
    format::json(response)
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/repos/{owner}/{repo}")
        .add("/fork", post(create))
        .add("/forks", get(list))
}
