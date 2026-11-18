//! Issues and labels API for a repository.
//!
//! Handlers stay thin: resolve the repo, call one model operation, render a DTO.

#![allow(clippy::unused_async)]

use loco_rs::prelude::*;
use serde::Deserialize;
use serde_json::json;

use crate::dtos::issues::{
    AddCommentParams, CommentResponse, CreateIssueParams, CreateLabelParams, IssueResponse,
    LabelResponse,
};
use crate::models::{events, issue_comments, issue_labels, issues, labels, repositories, users};

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

/// Public username for an issue/comment author.
async fn author_name(db: &sea_orm::DatabaseConnection, user_id: i64) -> Result<String> {
    users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .map(|user| user.username)
        .ok_or_else(|| Error::string("issue author is missing"))
}

/// Builds an issue DTO, including its labels and comment count.
async fn issue_response(
    db: &sea_orm::DatabaseConnection,
    issue: issues::Model,
) -> Result<IssueResponse> {
    let author = author_name(db, issue.user_id).await?;
    let labels = issue_labels::Entity::labels_for_issue(db, issue.id)
        .await?
        .into_iter()
        .map(|label| LabelResponse {
            id: label.id,
            name: label.name,
            color: label.color,
        })
        .collect();
    let comment_count = issue.comment_count(db).await?;
    Ok(IssueResponse {
        id: issue.id,
        number: issue.number,
        title: issue.title,
        body: issue.body,
        state: issue.state,
        author,
        comment_count,
        labels,
        created_at: issue.created_at.to_rfc3339(),
        updated_at: issue.updated_at.to_rfc3339(),
    })
}

#[derive(Debug, Deserialize)]
pub struct IssueListQuery {
    /// `open`, `closed`, or omitted for all issues.
    pub state: Option<String>,
}

/// `GET /api/repos/{owner}/{repo}/issues`.
#[debug_handler]
pub async fn list(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Query(query): Query<IssueListQuery>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let issues = issues::Model::find_by_repository(&ctx.db, row.id, query.state.as_deref()).await?;
    let mut response = Vec::with_capacity(issues.len());
    for issue in issues {
        response.push(issue_response(&ctx.db, issue).await?);
    }
    format::json(response)
}

/// `POST /api/repos/{owner}/{repo}/issues`.
#[debug_handler]
pub async fn create(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateIssueParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    let (issue, _labels) = issues::ActiveModel::create_for_repository(
        &ctx.db,
        row.id,
        user.id,
        &params.title,
        params.body.as_deref(),
        &params.label_ids,
    )
    .await?;
    crate::services::events::record(
        &ctx,
        row.id,
        user.id,
        events::kind::ISSUE_OPENED,
        Some(json!({ "number": issue.number })),
    )
    .await?;
    format::json(issue_response(&ctx.db, issue).await?)
}

/// `GET /api/repos/{owner}/{repo}/issues/{number}`.
#[debug_handler]
pub async fn show(
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let issue = issues::Model::find_by_number(&ctx.db, row.id, number).await?;
    format::json(issue_response(&ctx.db, issue).await?)
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
    if !row.can_write(&ctx.db, user.id).await? {
        return not_found();
    }
    let issue = issues::Model::find_by_number(&ctx.db, row.id, number).await?;
    let active = issues::ActiveModel {
        id: Set(issue.id),
        ..Default::default()
    };
    let updated = active.set_state(&ctx.db, state).await?;
    let kind = if state == issues::STATE_CLOSED {
        events::kind::ISSUE_CLOSED
    } else {
        events::kind::ISSUE_REOPENED
    };
    crate::services::events::record(
        ctx,
        row.id,
        user.id,
        kind,
        Some(json!({ "number": updated.number })),
    )
    .await?;
    format::json(issue_response(&ctx.db, updated).await?)
}

/// `POST /api/repos/{owner}/{repo}/issues/{number}/close`.
#[debug_handler]
pub async fn close(
    auth: auth::JWT,
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    set_state(auth, &ctx, &owner, &repo, number, issues::STATE_CLOSED).await
}

/// `POST /api/repos/{owner}/{repo}/issues/{number}/reopen`.
#[debug_handler]
pub async fn reopen(
    auth: auth::JWT,
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    set_state(auth, &ctx, &owner, &repo, number, issues::STATE_OPEN).await
}

/// `GET /api/repos/{owner}/{repo}/issues/{number}/comments`.
#[debug_handler]
pub async fn comments_list(
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let issue = issues::Model::find_by_number(&ctx.db, row.id, number).await?;
    let comments = issue_comments::Entity::find_by_issue(&ctx.db, issue.id).await?;
    let mut response = Vec::with_capacity(comments.len());
    for comment in comments {
        response.push(CommentResponse {
            id: comment.id,
            body: comment.body,
            author: author_name(&ctx.db, comment.user_id).await?,
            created_at: comment.created_at.to_rfc3339(),
        });
    }
    format::json(response)
}

/// `POST /api/repos/{owner}/{repo}/issues/{number}/comments`.
#[debug_handler]
pub async fn comment(
    auth: auth::JWT,
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    Json(params): Json<AddCommentParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    let issue = issues::Model::find_by_number(&ctx.db, row.id, number).await?;
    let comment =
        issue_comments::ActiveModel::create_for_issue(&ctx.db, issue.id, user.id, &params.body)
            .await?;
    format::json(CommentResponse {
        id: comment.id,
        body: comment.body,
        author: user.username,
        created_at: comment.created_at.to_rfc3339(),
    })
}

/// `GET /api/repos/{owner}/{repo}/labels`.
#[debug_handler]
pub async fn labels_list(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let labels = labels::Entity::find_by_repository(&ctx.db, row.id).await?;
    format::json(
        labels
            .into_iter()
            .map(|label| LabelResponse {
                id: label.id,
                name: label.name,
                color: label.color,
            })
            .collect::<Vec<_>>(),
    )
}

/// `POST /api/repos/{owner}/{repo}/labels`.
#[debug_handler]
pub async fn labels_create(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateLabelParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if !row.can_write(&ctx.db, user.id).await? {
        return not_found();
    }
    let label =
        labels::ActiveModel::create_for_repository(&ctx.db, row.id, &params.name, &params.color)
            .await?;
    format::json(LabelResponse {
        id: label.id,
        name: label.name,
        color: label.color,
    })
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/repos/{owner}/{repo}")
        .add("/issues", get(list))
        .add("/issues", post(create))
        .add("/issues/{number}", get(show))
        .add("/issues/{number}/close", post(close))
        .add("/issues/{number}/reopen", post(reopen))
        .add("/issues/{number}/comments", get(comments_list))
        .add("/issues/{number}/comments", post(comment))
        .add("/labels", get(labels_list))
        .add("/labels", post(labels_create))
}
