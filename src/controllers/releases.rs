//! Release and tag endpoints. Git tags are authoritative for the ref; Postgres
//! stores only the release metadata. Every access goes through the repository's
//! single `can` rule, so a release cannot expose a private repository.

#![allow(clippy::unused_async)]

use axum::{http::header, http::HeaderValue, response::IntoResponse};
use loco_rs::prelude::*;
use serde_json::json;

use crate::data::{git::GitRepo, settings::AppSettings};
use crate::dtos::releases::{CreateReleaseParams, ReleaseResponse, UpdateReleaseParams};
use crate::models::{events, releases, repositories, users};

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

/// Resolves a repository and requires write access, answering 404 on denial.
async fn find_writable_repo(
    ctx: &AppContext,
    owner: &str,
    repo: &str,
    user_id: i64,
) -> Result<repositories::Model> {
    let row = find_repo(ctx, owner, repo, Some(user_id)).await?;
    if !row.can_write(&ctx.db, user_id).await? {
        return not_found();
    }
    Ok(row)
}

fn open_git(ctx: &AppContext, owner: &str, repo: &str) -> Result<GitRepo> {
    let settings = AppSettings::from_context(ctx)?;
    GitRepo::open(&settings.repo_base_dir, owner, repo)
}

/// Builds the wire DTO, resolving the author and the tag's current commit from
/// git so the response never reports stale metadata.
async fn to_response(
    ctx: &AppContext,
    git: &GitRepo,
    release: &releases::Model,
) -> Result<ReleaseResponse> {
    let author = users::Entity::find_by_id(release.author_id)
        .one(&ctx.db)
        .await?
        .map(|user| user.owner_handle().to_owned())
        .unwrap_or_else(|| "unknown".to_owned());
    let commit_sha = git
        .tags()
        .await?
        .into_iter()
        .find(|tag| tag.name == release.tag_name)
        .map(|tag| tag.sha)
        .unwrap_or_default();
    Ok(ReleaseResponse {
        id: release.id,
        tag_name: release.tag_name.clone(),
        title: release.title.clone(),
        body: release.body.clone(),
        author,
        is_prerelease: release.is_prerelease,
        published_at: release
            .published_at
            .map(|date| date.to_rfc3339())
            .unwrap_or_default(),
        commit_sha,
    })
}

/// `GET /api/repos/{owner}/{repo}/releases`.
#[debug_handler]
pub async fn list(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let git = open_git(&ctx, &owner, &repo)?;
    let records = releases::Model::find_by_repository(&ctx.db, row.id).await?;
    let mut response = Vec::with_capacity(records.len());
    for release in records {
        response.push(to_response(&ctx, &git, &release).await?);
    }
    format::json(response)
}

/// `GET /api/repos/{owner}/{repo}/releases/latest`.
#[debug_handler]
pub async fn latest(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let git = open_git(&ctx, &owner, &repo)?;
    let Some(release) = releases::Model::find_latest(&ctx.db, row.id).await? else {
        return not_found();
    };
    format::json(to_response(&ctx, &git, &release).await?)
}

/// `GET /api/repos/{owner}/{repo}/releases/{id}`.
#[debug_handler]
pub async fn show(
    Path((owner, repo, id)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let release = releases::Entity::find_by_id(id)
        .one(&ctx.db)
        .await?
        .filter(|release| release.repository_id == row.id)
        .ok_or(Error::NotFound)?;
    let git = open_git(&ctx, &owner, &repo)?;
    format::json(to_response(&ctx, &git, &release).await?)
}

/// `POST /api/repos/{owner}/{repo}/releases`.
///
/// Creates the git tag first with a compare-and-swap, then the metadata row.
/// If the row insert fails the tag is removed again, so the two stores cannot
/// be left half-updated by a normal failure.
#[debug_handler]
pub async fn create(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateReleaseParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_writable_repo(&ctx, &owner, &repo, user.id).await?;
    let git = open_git(&ctx, &owner, &repo)?;

    let commit_sha = git.create_tag(&params.tag_name, &params.target).await?;
    let release = match releases::ActiveModel::create(
        &ctx.db,
        row.id,
        user.id,
        &params.tag_name,
        &params.title,
        params.body.as_deref(),
        params.is_prerelease,
    )
    .await
    {
        Ok(release) => release,
        Err(err) => {
            // Best-effort rollback: the tag is ours because the CAS above
            // refused to overwrite an existing one.
            let _ = git.delete_tag(&params.tag_name).await;
            return Err(err);
        }
    };

    crate::services::events::record(
        &ctx,
        row.id,
        user.id,
        events::kind::RELEASE_PUBLISHED,
        Some(json!({ "tag": release.tag_name, "title": release.title })),
    )
    .await?;

    let mut response = to_response(&ctx, &git, &release).await?;
    response.commit_sha = commit_sha;
    format::json(response)
}

/// `PATCH /api/repos/{owner}/{repo}/releases/{id}`.
#[debug_handler]
pub async fn update(
    auth: auth::JWT,
    Path((owner, repo, id)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateReleaseParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_writable_repo(&ctx, &owner, &repo, user.id).await?;
    let release = releases::Entity::find_by_id(id)
        .one(&ctx.db)
        .await?
        .filter(|release| release.repository_id == row.id)
        .ok_or(Error::NotFound)?;
    let release = releases::ActiveModel::update_details(
        &ctx.db,
        release,
        &params.title,
        params.body.as_deref(),
        params.is_prerelease,
    )
    .await?;
    crate::services::events::record(
        &ctx,
        row.id,
        user.id,
        events::kind::RELEASE_UPDATED,
        Some(json!({ "tag": release.tag_name })),
    )
    .await?;
    let git = open_git(&ctx, &owner, &repo)?;
    format::json(to_response(&ctx, &git, &release).await?)
}

/// `DELETE /api/repos/{owner}/{repo}/releases/{id}`.
///
/// Removes the metadata row and then the git tag. If the tag removal fails the
/// release is already gone; the orphan tag can be deleted from the Tags list.
#[debug_handler]
pub async fn remove(
    auth: auth::JWT,
    Path((owner, repo, id)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_writable_repo(&ctx, &owner, &repo, user.id).await?;
    let release = releases::Entity::find_by_id(id)
        .one(&ctx.db)
        .await?
        .filter(|release| release.repository_id == row.id)
        .ok_or(Error::NotFound)?;
    let active: releases::ActiveModel = release.clone().into();
    active.delete(&ctx.db).await?;
    let git = open_git(&ctx, &owner, &repo)?;
    if let Err(err) = git.delete_tag(&release.tag_name).await {
        tracing::warn!(%err, tag = %release.tag_name, "release deleted but tag remains");
    }
    crate::services::events::record(
        &ctx,
        row.id,
        user.id,
        events::kind::RELEASE_DELETED,
        Some(json!({ "tag": release.tag_name })),
    )
    .await?;
    format::empty_json()
}

/// `GET /api/repos/{owner}/{repo}/tags`.
#[debug_handler]
pub async fn list_tags(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    find_repo(&ctx, &owner, &repo, viewer).await?;
    let git = open_git(&ctx, &owner, &repo)?;
    format::json(git.tags().await?)
}

/// `DELETE /api/repos/{owner}/{repo}/tags/{tag}`.
#[debug_handler]
pub async fn delete_tag(
    auth: auth::JWT,
    Path((owner, repo, tag)): Path<(String, String, String)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_writable_repo(&ctx, &owner, &repo, user.id).await?;
    let git = open_git(&ctx, &owner, &repo)?;
    git.delete_tag(&tag).await?;
    if let Some(release) = releases::Model::find_by_tag(&ctx.db, row.id, &tag).await? {
        let active: releases::ActiveModel = release.into();
        active.delete(&ctx.db).await?;
    }
    crate::services::events::record(
        &ctx,
        row.id,
        user.id,
        events::kind::TAG_DELETED,
        Some(json!({ "tag": tag })),
    )
    .await?;
    format::empty_json()
}

/// `GET /api/repos/{owner}/{repo}/archive/{reference}.tar.gz` (the whole
/// filename is the `{reference}` segment).
#[debug_handler]
pub async fn archive(
    Path((owner, repo, reference)): Path<(String, String, String)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    find_repo(&ctx, &owner, &repo, viewer).await?;
    // The whole filename is one path segment (Loco rejects a param followed
    // by a literal suffix), so accept both `<ref>` and `<ref>.tar.gz`.
    let reference = reference.strip_suffix(".tar.gz").unwrap_or(&reference);
    let git = open_git(&ctx, &owner, &repo)?;
    let bytes = git.archive(reference).await?;
    let mut response = bytes.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/gzip"),
    );
    let filename = format!("attachment; filename=\"{owner}-{repo}-{reference}.tar.gz\"");
    if let Ok(value) = HeaderValue::from_str(&filename) {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    Ok(response)
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/repos/{owner}/{repo}")
        .add("/releases", get(list))
        .add("/releases", post(create))
        .add("/releases/latest", get(latest))
        .add("/releases/{id}", get(show))
        .add("/releases/{id}", patch(update))
        .add("/releases/{id}", delete(remove))
        .add("/tags", get(list_tags))
        .add("/tags/{tag}", delete(delete_tag))
        .add("/archive/{reference}", get(archive))
}
