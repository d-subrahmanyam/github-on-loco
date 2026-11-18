//! Collaborator management for a repository. Only admins (and the owner) may
//! change grants; anyone who can read the repo may view them.

#![allow(clippy::unused_async)]

use loco_rs::prelude::*;
use serde_json::json;

use crate::dtos::collaborators::{
    AddCollaboratorParams, CollaboratorResponse, UpdateCollaboratorParams,
};
use crate::models::{collaborators, events, repositories, users};

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

/// Resolves the repository and enforces read visibility.
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

/// Builds a collaborator DTO, resolving the user's current username.
async fn collaborator_response(
    db: &sea_orm::DatabaseConnection,
    grant: collaborators::Model,
) -> Result<CollaboratorResponse> {
    let user = users::Entity::find_by_id(grant.user_id)
        .one(db)
        .await?
        .ok_or_else(|| Error::string("collaborator user is missing"))?;
    Ok(CollaboratorResponse {
        id: grant.id,
        user_id: user.id,
        username: user.username,
        role: grant.role,
    })
}

/// `GET /api/repos/{owner}/{repo}/collaborators`.
#[debug_handler]
pub async fn list(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let viewer = viewer_id(&ctx, viewer).await?;
    let row = find_repo(&ctx, &owner, &repo, viewer).await?;
    let grants = collaborators::Entity::find_by_repository(&ctx.db, row.id).await?;
    let mut response = Vec::with_capacity(grants.len());
    for grant in grants {
        response.push(collaborator_response(&ctx.db, grant).await?);
    }
    format::json(response)
}

/// `POST /api/repos/{owner}/{repo}/collaborators`.
#[debug_handler]
pub async fn add(
    auth: auth::JWT,
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Json(params): Json<AddCollaboratorParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if !row.can_admin(&ctx.db, user.id).await? {
        return not_found();
    }

    let invited = users::Model::find_by_username(&ctx.db, &params.username).await?;
    if invited.id == row.user_id {
        return bad_request("the repository owner already has every permission");
    }
    let grant = collaborators::ActiveModel::add(&ctx.db, row.id, invited.id, &params.role).await?;
    events::Entity::record(
        &ctx.db,
        row.id,
        user.id,
        events::kind::COLLABORATOR_ADDED,
        Some(json!({ "target": invited.username, "role": params.role })),
    )
    .await?;
    format::json(collaborator_response(&ctx.db, grant).await?)
}

/// `PATCH /api/repos/{owner}/{repo}/collaborators/{user_id}`.
#[debug_handler]
pub async fn update(
    auth: auth::JWT,
    Path((owner, repo, user_id)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateCollaboratorParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if !row.can_admin(&ctx.db, user.id).await? {
        return not_found();
    }

    let grant = collaborators::Entity::find_for(&ctx.db, row.id, user_id)
        .await?
        .ok_or_else(|| Error::string("user is not a collaborator"))?;
    let active = collaborators::ActiveModel {
        id: Set(grant.id),
        ..Default::default()
    };
    let updated = active.set_role(&ctx.db, &params.role).await?;
    format::json(collaborator_response(&ctx.db, updated).await?)
}

/// `DELETE /api/repos/{owner}/{repo}/collaborators/{user_id}`.
#[debug_handler]
pub async fn remove(
    auth: auth::JWT,
    Path((owner, repo, user_id)): Path<(String, String, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let row = find_repo(&ctx, &owner, &repo, Some(user.id)).await?;
    if !row.can_admin(&ctx.db, user.id).await? {
        return not_found();
    }
    let target = users::Entity::find_by_id(user_id)
        .one(&ctx.db)
        .await?
        .map(|user| user.username)
        .unwrap_or_else(|| "unknown".to_owned());
    collaborators::Entity::remove(&ctx.db, row.id, user_id).await?;
    events::Entity::record(
        &ctx.db,
        row.id,
        user.id,
        events::kind::COLLABORATOR_REMOVED,
        Some(json!({ "target": target })),
    )
    .await?;
    format::empty_json()
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/repos/{owner}/{repo}")
        .add("/collaborators", get(list))
        .add("/collaborators", post(add))
        .add("/collaborators/{user_id}", patch(update))
        .add("/collaborators/{user_id}", delete(remove))
}
