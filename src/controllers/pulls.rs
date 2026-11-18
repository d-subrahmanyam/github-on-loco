//! Pull request API for a repository.
//!
//! Handlers stay thin: resolve the repo, call one model operation, render a DTO.
//! Merging is the one exception — it enqueues a worker because it touches git.

#![allow(clippy::unused_async)]

use loco_rs::prelude::*;
use serde::Deserialize;
use serde_json::json;

use crate::data::{git::GitRepo, settings::AppSettings};
use crate::dtos::pulls::{
    AddReviewCommentParams, CompareResult, CreatePullRequestParams, PullRequestResponse,
    ReviewCommentResponse,
};
use crate::models::{events, pull_requests, repositories, review_comments, users};
use crate::workers::merge_pull_request::{Worker as MergeWorker, WorkerArgs as MergeArgs};

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

/// Resolves a repository and enforces read visibility with a 404 for private
/// repositories the viewer may not see.
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

/// Public username of a PR or comment author.
async fn author_name(db: &sea_orm::DatabaseConnection, user_id: i64) -> Result<String> {
    users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .map(|user| user.username)
        .ok_or_else(|| Error::string("pull request author is missing"))
}

/// Open the bare repository for a metadata row.
fn open_git(ctx: &AppContext, owner: &str, repo: &repositories::Model) -> Result<GitRepo> {
    let settings = AppSettings::from_context(ctx)?;
    GitRepo::open(&settings.repo_base_dir, owner, repo.display_name()?)
}

/// Builds a PR DTO.
async fn pr_response(
    db: &sea_orm::DatabaseConnection,
    pr: pull_requests::Model,
) -> Result<PullRequestResponse> {
    let comment_count = pr.comment_count(db).await?;
    Ok(PullRequestResponse {
        id: pr.id,
        number: pr.number,
        title: pr.title,
        body: pr.body,
        state: pr.state,
        author: author_name(db, pr.user_id).await?,
        head_ref: pr.head_ref,
        head_sha: pr.head_sha,
        base_ref: pr.base_ref,
        comment_count,
        merged_at: pr.merged_at.map(|value| value.to_rfc3339()),
        merged_commit_sha: pr.merged_commit_sha,
        created_at: pr.created_at.to_rfc3339(),
        updated_at: pr.updated_at.to_rfc3339(),
    })
}

/// Builds a review comment DTO.
async fn comment_response(
    db: &sea_orm::DatabaseConnection,
    comment: review_comments::Model,
) -> Result<ReviewCommentResponse> {
    Ok(ReviewCommentResponse {
        id: comment.id,
        body: comment.body,
        author: author_name(db, comment.user_id).await?,
        path: comment.path,
        line: comment.line,
        created_at: comment.created_at.to_rfc3339(),
    })
}

#[derive(Debug, Deserialize)]
pub struct PullListQuery {
    /// `open`, `closed`, `merged`, or omitted for all.
    pub state: Option<String>,
}

/// `GET /api/repos/{owner}/{repo}/pulls`.
#[debug_handler]
pub async fn list(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Query(query): Query<PullListQuery>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let prs =
        pull_requests::Model::find_by_repository(&ctx.db, row.id, query.state.as_deref()).await?;
    let mut response = Vec::with_capacity(prs.len());
    for pr in prs {
        response.push(pr_response(&ctx.db, pr).await?);
    }
    format::json(response)
}

#[derive(Debug, Deserialize)]
pub struct CompareQuery {
    /// Head branch to compare.
    pub head: String,
    /// Base branch; defaults to the repository's HEAD branch.
    pub base: Option<String>,
}

/// `GET /api/repos/{owner}/{repo}/pulls/compare?head=...&base=...`.
#[debug_handler]
pub async fn compare(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Query(query): Query<CompareQuery>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let git = open_git(&ctx, &owner, &row)?;
    let base = match query.base {
        Some(base) if !base.is_empty() => base,
        _ => git.default_branch().await?,
    };
    let result: CompareResult = git.compare(&base, &query.head).await?;
    format::json(result)
}

/// `POST /api/repos/{owner}/{repo}/pulls`.
#[debug_handler]
pub async fn create(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreatePullRequestParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if !row.can_write(&ctx.db, user.id).await? {
        return not_found();
    }
    let git = open_git(&ctx, &owner, &row)?;
    // Resolve once at creation: the diff and the eventual merge are pinned to
    // this SHA even if the head branch is force-pushed later.
    let head_sha = git.resolve(&params.head_ref).await?;
    let pr = pull_requests::ActiveModel::create_for_repository(
        &ctx.db,
        row.id,
        user.id,
        &params.title,
        params.body.as_deref(),
        &params.head_ref,
        &head_sha,
        &params.base_ref,
    )
    .await?;
    events::Entity::record(
        &ctx.db,
        row.id,
        user.id,
        events::kind::PR_OPENED,
        Some(json!({ "number": pr.number })),
    )
    .await?;
    format::json(pr_response(&ctx.db, pr).await?)
}

/// `GET /api/repos/{owner}/{repo}/pulls/{number}`.
#[debug_handler]
pub async fn show(
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let pr = pull_requests::Model::find_by_number(&ctx.db, row.id, number).await?;
    format::json(pr_response(&ctx.db, pr).await?)
}

/// True for a repository writer (or admin/owner) or the PR author.
async fn can_modify_pr(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    repo: &repositories::Model,
    pr: &pull_requests::Model,
) -> Result<bool> {
    Ok(repo.can_write(db, user_id).await? || pr.user_id == user_id)
}

/// Shared handler for the close/reopen transitions.
async fn set_state(
    auth: auth::JWT,
    ctx: &AppContext,
    owner: &str,
    repo: &str,
    number: i64,
    state: &str,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(ctx, owner, repo, Some(user.id)).await?;
    let pr = pull_requests::Model::find_by_number(&ctx.db, row.id, number).await?;
    if !can_modify_pr(&ctx.db, user.id, &row, &pr).await? {
        return not_found();
    }
    if pr.is_merged() {
        return bad_request("a merged pull request cannot change state");
    }
    let active = pull_requests::ActiveModel {
        id: Set(pr.id),
        ..Default::default()
    };
    let updated = active.set_state(&ctx.db, state).await?;
    format::json(pr_response(&ctx.db, updated).await?)
}

/// `POST /api/repos/{owner}/{repo}/pulls/{number}/close`.
#[debug_handler]
pub async fn close(
    auth: auth::JWT,
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    set_state(
        auth,
        &ctx,
        &owner,
        &repo,
        number,
        pull_requests::STATE_CLOSED,
    )
    .await
}

/// `POST /api/repos/{owner}/{repo}/pulls/{number}/reopen`.
#[debug_handler]
pub async fn reopen(
    auth: auth::JWT,
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    set_state(auth, &ctx, &owner, &repo, number, pull_requests::STATE_OPEN).await
}

/// `POST /api/repos/{owner}/{repo}/pulls/{number}/merge`.
#[debug_handler]
pub async fn merge(
    auth: auth::JWT,
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if !row.can_write(&ctx.db, user.id).await? {
        return not_found();
    }
    let mut pr = pull_requests::Model::find_by_number(&ctx.db, row.id, number).await?;
    if !pr.is_open() {
        return bad_request("only open pull requests can be merged");
    }

    // Defensive refresh: the push handler normally keeps head_sha current, but
    // a PR may have been reopened after a push that bypassed it. Re-resolve
    // here so the worker always merges the branch's actual tip.
    let git = open_git(&ctx, &owner, &row)?;
    if let Ok(sha) = git.resolve(&pr.head_ref).await {
        if sha != pr.head_sha {
            let active = pull_requests::ActiveModel {
                id: Set(pr.id),
                ..Default::default()
            };
            pr = active.set_head_sha(&ctx.db, &sha).await?;
        }
    }

    // Reload after enqueue: in ForegroundBlocking mode (tests/dev) the worker
    // has already run inline by the time `perform_later` returns.
    MergeWorker::perform_later(
        &ctx,
        MergeArgs {
            pull_request_id: pr.id,
            actor_id: user.id,
        },
    )
    .await?;
    let pr = pull_requests::Entity::find_by_id(pr.id)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::string("pull request disappeared during merge"))?;
    format::json(pr_response(&ctx.db, pr).await?)
}

/// `GET /api/repos/{owner}/{repo}/pulls/{number}/comments`.
#[debug_handler]
pub async fn comments_list(
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let pr = pull_requests::Model::find_by_number(&ctx.db, row.id, number).await?;
    let comments = review_comments::Entity::find_by_pr(&ctx.db, pr.id).await?;
    let mut response = Vec::with_capacity(comments.len());
    for comment in comments {
        response.push(comment_response(&ctx.db, comment).await?);
    }
    format::json(response)
}

/// `POST /api/repos/{owner}/{repo}/pulls/{number}/comments`.
#[debug_handler]
pub async fn comment(
    auth: auth::JWT,
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    Json(params): Json<AddReviewCommentParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    let pr = pull_requests::Model::find_by_number(&ctx.db, row.id, number).await?;
    let comment = review_comments::ActiveModel::create_for_pr(
        &ctx.db,
        pr.id,
        user.id,
        &params.body,
        params.path.as_deref(),
        params.line,
    )
    .await?;
    format::json(comment_response(&ctx.db, comment).await?)
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/repos/{owner}/{repo}")
        // The static `compare` segment is registered before `{number}` so it
        // is never mistaken for a numeric PR number.
        .add("/pulls/compare", get(compare))
        .add("/pulls", get(list))
        .add("/pulls", post(create))
        .add("/pulls/{number}", get(show))
        .add("/pulls/{number}/close", post(close))
        .add("/pulls/{number}/reopen", post(reopen))
        .add("/pulls/{number}/merge", post(merge))
        .add("/pulls/{number}/comments", get(comments_list))
        .add("/pulls/{number}/comments", post(comment))
}
