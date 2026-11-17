//! Personal access token management for the current user.
//!
//! The plaintext token is shown once at creation; only its hash is stored.

#![allow(clippy::unused_async)]

use chrono::{Duration, Utc};
use loco_rs::prelude::*;

use crate::dtos::tokens::{CreateTokenParams, NewTokenResponse, TokenResponse};
use crate::models::{personal_access_tokens, users};

fn to_response(token: personal_access_tokens::Model) -> TokenResponse {
    TokenResponse {
        id: token.id,
        name: token.name,
        scopes: token.scopes,
        expires_at: token.expires_at.map(|value| value.to_rfc3339()),
        last_used_at: token.last_used_at.map(|value| value.to_rfc3339()),
        created_at: token.created_at.to_rfc3339(),
    }
}

/// `GET /api/tokens` — the current user's tokens (never their hashes).
#[debug_handler]
pub async fn list(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let tokens = personal_access_tokens::Entity::find_by_user(&ctx.db, user.id).await?;
    format::json(tokens.into_iter().map(to_response).collect::<Vec<_>>())
}

/// `POST /api/tokens` — issue a new token and return its plaintext once.
#[debug_handler]
pub async fn create(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateTokenParams>,
) -> Result<Response> {
    if params.name.trim().is_empty() {
        return bad_request("token name must not be empty");
    }
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let expires_at = params
        .expires_in_days
        .map(|days| (Utc::now() + Duration::days(days)).into());

    let (stored, plaintext) = personal_access_tokens::ActiveModel::issue(
        &ctx.db,
        user.id,
        params.name.trim(),
        params.scopes.as_deref(),
        expires_at,
    )
    .await?;

    format::json(NewTokenResponse {
        token: plaintext,
        info: to_response(stored),
    })
}

/// `DELETE /api/tokens/{id}` — revoke one of the current user's tokens.
#[debug_handler]
pub async fn revoke(
    auth: auth::JWT,
    Path(id): Path<i64>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    personal_access_tokens::Entity::revoke(&ctx.db, id, user.id).await?;
    format::empty()
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/tokens")
        .add("/", get(list))
        .add("/", post(create))
        .add("/{id}", delete(revoke))
}
