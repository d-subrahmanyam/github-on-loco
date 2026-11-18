//! Repository management endpoints.
//!
//! Handlers here only parse input, call one or two model methods and render a
//! DTO. All naming, ownership and visibility rules live on the model.

use axum::debug_handler;
use loco_rs::prelude::*;
use sea_orm::ActiveModelTrait;
use std::path::Path;
use tokio::process::Command;

use crate::data::settings::AppSettings;
use crate::dtos::repositories::{CreateRepoParams, RepoResponse};
use crate::models::{organizations, repositories as repo_model, users};

/// Initialises an empty bare repository on disk at `path`.
async fn init_bare_repo(path: &Path) -> Result<()> {
    tokio::fs::create_dir_all(path)
        .await
        .map_err(|e| Error::string(&format!("failed to create repo directory: {e}")))?;

    let status = Command::new("git")
        .arg("init")
        .arg("--bare")
        .arg(path)
        .status()
        .await
        .map_err(|e| Error::string(&format!("failed to execute git init: {e}")))?;

    if !status.success() {
        return Err(Error::string("git init --bare failed"));
    }
    Ok(())
}

/// Builds the public clone URL from the configured server host and port.
fn clone_url(ctx: &AppContext, path: &str) -> String {
    format!("{}{path}", ctx.config.server.full_url())
}

/// `POST /api/repositories` — create a repository and its bare git directory.
#[debug_handler]
pub async fn create(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateRepoParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    // The handle in the URL (and on disk) is the org's when one is selected.
    // Only org owners may create repositories under its namespace.
    let (owner, repo) = match params.organization.as_deref() {
        Some(handle) if !handle.is_empty() => {
            let org = organizations::Entity::find_by_handle(&ctx.db, handle).await?;
            if !org.is_owner(&ctx.db, user.id).await? {
                return not_found();
            }
            let repo = repo_model::ActiveModel::create_for_organization(
                &ctx.db,
                org.id,
                user.id,
                &params.name,
                params.description.as_deref(),
                params.is_private,
            )
            .await?;
            (org.handle.clone(), repo)
        }
        _ => {
            let repo = repo_model::ActiveModel::create_for_user(
                &ctx.db,
                user.id,
                &params.name,
                params.description.as_deref(),
                params.is_private,
            )
            .await?;
            (user.owner_handle().to_owned(), repo)
        }
    };

    let settings = AppSettings::from_context(&ctx)?;
    let dir = repo_model::repo_dir(&settings.repo_base_dir, &owner, repo.display_name()?)?;

    // Disk and DB are two stores. If git init fails, remove the row so we do
    // not leave metadata pointing at a repository that does not exist.
    if let Err(err) = init_bare_repo(&dir).await {
        tracing::error!(error = %err, path = %dir.display(), "rolling back repository row");
        let active: repo_model::ActiveModel = repo.clone().into();
        active.delete(&ctx.db).await?;
        return Err(err);
    }

    format::json(RepoResponse {
        id: repo.id,
        owner: owner.clone(),
        name: repo.display_name()?.to_owned(),
        description: repo.description.clone(),
        is_private: repo.is_private(),
        clone_url: clone_url(&ctx, &repo.clone_path(&owner)?),
        viewer_role: repo_model::ROLE_OWNER.to_owned(),
        star_count: 0,
        watch_count: 0,
        starred: false,
        watching: false,
        fork_count: 0,
        parent: None,
    })
}

/// `GET /api/repositories` — repositories owned by the current user.
#[debug_handler]
pub async fn list(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let repos = repo_model::Model::find_by_user(&ctx.db, user.id).await?;

    let response: Vec<RepoResponse> = repos
        .iter()
        .map(|repo| {
            let name = repo.display_name()?.to_owned();
            Ok(RepoResponse {
                id: repo.id,
                owner: user.owner_handle().to_owned(),
                name: name.clone(),
                description: repo.description.clone(),
                is_private: repo.is_private(),
                clone_url: clone_url(&ctx, &format!("/{}/{name}.git", user.owner_handle())),
                viewer_role: repo_model::ROLE_OWNER.to_owned(),
                star_count: 0,
                watch_count: 0,
                starred: false,
                watching: false,
                fork_count: 0,
                parent: None,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    format::json(response)
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/repositories")
        .add("/", post(create))
        .add("/", get(list))
}
