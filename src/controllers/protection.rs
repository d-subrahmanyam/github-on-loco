//! Branch protection settings and pull request reviews.
//!
//! Settings are admin-only; reviews require write access. Both decisions reuse
//! the repository's single `can` rule — there is no second permission system.

#![allow(clippy::unused_async)]

use loco_rs::prelude::*;
use serde::Deserialize;
use serde_json::json;

use crate::dtos::protection::{
    BranchProtectionResponse, PrReviewResponse, SetBranchProtectionParams, SetReviewParams,
};
use crate::models::{
    branch_protection_rules as protection, events, pr_reviews, pull_requests, repositories, users,
};

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

fn protection_response(row: &protection::Model) -> BranchProtectionResponse {
    BranchProtectionResponse {
        id: row.id,
        branch: row.branch.clone(),
        require_pull_request: row.require_pull_request(),
        required_approvals: row.required_approvals(),
    }
}

async fn review_response(
    db: &sea_orm::DatabaseConnection,
    row: pr_reviews::Model,
) -> Result<PrReviewResponse> {
    let author = users::Entity::find_by_id(row.user_id)
        .one(db)
        .await?
        .map(|user| user.username)
        .ok_or_else(|| Error::string("reviewer is missing"))?;
    Ok(PrReviewResponse {
        id: row.id,
        author,
        state: row.state,
        body: row.body,
        created_at: row.created_at.to_rfc3339(),
    })
}

/// `GET /api/repos/{owner}/{repo}/branch-protection`.
#[debug_handler]
pub async fn list(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if !row.can_admin(&ctx.db, user.id).await? {
        return not_found();
    }
    let rules = protection::Entity::find_by_repository(&ctx.db, row.id).await?;
    format::json(rules.iter().map(protection_response).collect::<Vec<_>>())
}

/// `POST /api/repos/{owner}/{repo}/branch-protection` (upsert).
#[debug_handler]
pub async fn set(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Json(params): Json<SetBranchProtectionParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if !row.can_admin(&ctx.db, user.id).await? {
        return not_found();
    }
    let rule = protection::ActiveModel::upsert(
        &ctx.db,
        row.id,
        &params.branch,
        params.require_pull_request,
        params.required_approvals,
    )
    .await?;
    format::json(protection_response(&rule))
}

#[derive(Debug, Deserialize)]
pub struct BranchQuery {
    pub branch: String,
}

/// `DELETE /api/repos/{owner}/{repo}/branch-protection?branch=...`.
#[debug_handler]
pub async fn remove(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Query(query): Query<BranchQuery>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if !row.can_admin(&ctx.db, user.id).await? {
        return not_found();
    }
    protection::ActiveModel::remove(&ctx.db, row.id, &query.branch).await?;
    format::empty_json()
}

/// `GET /api/repos/{owner}/{repo}/pulls/{number}/reviews`.
#[debug_handler]
pub async fn reviews(
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let pr = pull_requests::Model::find_by_number(&ctx.db, row.id, number).await?;
    let records = pr_reviews::Entity::find_by_pr(&ctx.db, pr.id).await?;
    let mut response = Vec::with_capacity(records.len());
    for record in records {
        response.push(review_response(&ctx.db, record).await?);
    }
    format::json(response)
}

/// `POST /api/repos/{owner}/{repo}/pulls/{number}/reviews`.
#[debug_handler]
pub async fn set_review(
    auth: auth::JWT,
    Path((owner, repo, number)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    Json(params): Json<SetReviewParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if !row.can_write(&ctx.db, user.id).await? {
        return not_found();
    }
    let pr = pull_requests::Model::find_by_number(&ctx.db, row.id, number).await?;
    if pr.user_id == user.id {
        return bad_request("you cannot review your own pull request");
    }
    if !pr.is_open() {
        return bad_request("only open pull requests can be reviewed");
    }
    let review = pr_reviews::ActiveModel::set_review(
        &ctx.db,
        pr.id,
        user.id,
        &params.state,
        params.body.as_deref(),
    )
    .await?;
    let kind = match params.state.as_str() {
        pr_reviews::state::APPROVED => events::kind::PR_APPROVED,
        pr_reviews::state::CHANGES_REQUESTED => events::kind::PR_CHANGES_REQUESTED,
        _ => events::kind::PR_REVIEWED,
    };
    events::Entity::record(
        &ctx.db,
        row.id,
        user.id,
        kind,
        Some(json!({ "number": pr.number })),
    )
    .await?;
    format::json(review_response(&ctx.db, review).await?)
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/repos/{owner}/{repo}")
        .add("/branch-protection", get(list))
        .add("/branch-protection", post(set))
        .add("/branch-protection", delete(remove))
        .add("/pulls/{number}/reviews", get(reviews))
        .add("/pulls/{number}/reviews", post(set_review))
}
