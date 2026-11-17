//! Read-only repository browsing API.
//!
//! These endpoints answer the SPA's questions — which branches exist, what
//! happened recently, what is in this tree — by asking [`GitRepo`]. The SQL row
//! supplies metadata; the git repository supplies history.

#![allow(clippy::unused_async)]

use axum::{
    body::Body,
    http::{header, StatusCode},
};
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::data::{git::GitRepo, settings::AppSettings};
use crate::dtos::repositories::{Branch, Commit, CommitDetail, RepoResponse, TreeEntry};
use crate::models::{branches, commits, repositories as repo_model, users};

/// Resolves an optional JWT to a user id. Anonymous requests are allowed.
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

/// Loads the metadata row and opens the bare repository, enforcing visibility.
async fn open_repo(
    ctx: &AppContext,
    owner: &str,
    repo: &str,
    viewer: Option<i64>,
) -> Result<(repo_model::Model, GitRepo)> {
    let settings = AppSettings::from_context(ctx)?;
    let row = repo_model::Model::find_by_owner_and_name(&ctx.db, owner, repo).await?;
    // 404 rather than 401: do not reveal that a private repository exists.
    if !row.can_read(viewer) {
        return not_found();
    }
    let git = GitRepo::open(&settings.repo_base_dir, owner, repo)?;
    Ok((row, git))
}

/// Picks the requested revision, falling back to the repository default branch.
async fn reference_or_default(git: &GitRepo, reference: Option<&str>) -> Result<String> {
    match reference {
        Some(value) if !value.is_empty() => Ok(value.to_owned()),
        _ => git.default_branch().await,
    }
}

/// `GET /api/repos/{owner}/{repo}` — repository metadata.
#[debug_handler]
pub async fn overview(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let (row, _git) = open_repo(&ctx, &owner, &repo, viewer).await?;
    format::json(RepoResponse {
        id: row.id,
        owner: owner.clone(),
        name: row.display_name()?.to_owned(),
        description: row.description.clone(),
        is_private: row.is_private(),
        clone_url: format!(
            "{}{}",
            ctx.config.server.full_url(),
            row.clone_path(&owner)?
        ),
    })
}

/// `GET /api/repos/{owner}/{repo}/branches`.
#[debug_handler]
pub async fn branches(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let (row, git) = open_repo(&ctx, &owner, &repo, viewer).await?;

    // Git is authoritative; the table is only a read-through cache. A cache
    // error must never turn a browsing request into a 500.
    let cached = branches::Entity::find_by_repository(&ctx.db, row.id)
        .await
        .unwrap_or_default();
    let branches: Vec<Branch> = if cached.is_empty() {
        let fresh = git.branches().await?;
        let _ = branches::Entity::replace_for_repository(&ctx.db, row.id, &fresh).await;
        fresh
    } else {
        cached
            .into_iter()
            .map(|entry| Branch {
                name: entry.name,
                sha: entry.sha,
            })
            .collect()
    };
    format::json(branches)
}

#[derive(Debug, Deserialize)]
pub struct CommitsQuery {
    #[serde(rename = "ref", default)]
    reference: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    20
}

/// `GET /api/repos/{owner}/{repo}/commits?ref=main&limit=20`.
#[debug_handler]
pub async fn commits(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Query(query): Query<CommitsQuery>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let (row, git) = open_repo(&ctx, &owner, &repo, viewer).await?;
    let reference = reference_or_default(&git, query.reference.as_deref()).await?;

    // Only the default branch has a cache key in this table. Explicit refs
    // always read git so we can never serve another branch's history.
    let use_cache = query.reference.is_none();
    let cached = if use_cache {
        commits::Entity::find_by_repository(&ctx.db, row.id)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let commits: Vec<Commit> = if cached.is_empty() {
        let fresh = git.commits(&reference, query.limit).await?;
        if use_cache {
            let _ = commits::Entity::replace_for_repository(&ctx.db, row.id, &fresh).await;
        }
        fresh
    } else {
        cached
            .into_iter()
            .take(query.limit)
            .map(|entry| Commit {
                sha: entry.sha,
                author: entry.author,
                committed_at: entry.committed_at.to_rfc3339(),
                message: entry.message,
            })
            .collect()
    };
    format::json(commits)
}

/// `GET /api/repos/{owner}/{repo}/commit/{sha}`.
#[debug_handler]
pub async fn commit(
    Path((owner, repo, sha)): Path<(String, String, String)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let (_row, git) = open_repo(&ctx, &owner, &repo, viewer).await?;
    let detail: CommitDetail = git.commit(&sha).await?;
    format::json(detail)
}

#[derive(Debug, Deserialize)]
pub struct TreeQuery {
    #[serde(rename = "ref", default)]
    reference: Option<String>,
    #[serde(default)]
    path: Option<String>,
}

/// `GET /api/repos/{owner}/{repo}/tree?ref=main&path=src`.
#[debug_handler]
pub async fn tree(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Query(query): Query<TreeQuery>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let (_row, git) = open_repo(&ctx, &owner, &repo, viewer).await?;
    let reference = reference_or_default(&git, query.reference.as_deref()).await?;
    let entries: Vec<TreeEntry> = git
        .tree(&reference, query.path.as_deref().unwrap_or(""))
        .await?;
    format::json(entries)
}

/// Guesses a response content type for raw file requests.
fn content_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".md") || path.ends_with(".txt") {
        "text/plain; charset=utf-8"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else {
        "application/octet-stream"
    }
}

/// `GET /api/repos/{owner}/{repo}/raw?ref=main&path=README.md`.
#[debug_handler]
pub async fn raw(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Query(query): Query<TreeQuery>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let (_row, git) = open_repo(&ctx, &owner, &repo, viewer).await?;
    let reference = reference_or_default(&git, query.reference.as_deref()).await?;
    let path = query.path.as_deref().unwrap_or("");
    let bytes = git.blob(&reference, path).await?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type(path))],
        Body::from(bytes),
    )
        .into_response())
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/repos/{owner}/{repo}")
        .add("/", get(overview))
        .add("/branches", get(branches))
        .add("/commits", get(commits))
        .add("/commit/{sha}", get(commit))
        .add("/tree", get(tree))
        .add("/raw", get(raw))
}

#[cfg(test)]
mod tests {
    use super::content_type;

    #[test]
    fn content_type_covers_common_extensions() {
        assert_eq!(content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(content_type("README.md"), "text/plain; charset=utf-8");
        assert_eq!(content_type("data.json"), "application/json");
        assert_eq!(content_type("image.gif"), "application/octet-stream");
    }
}
