//! Git smart-HTTP transport for clone, fetch and push.
//!
//! Loco handles REST; git has its own wire protocol. Rather than reimplement
//! it, these handlers translate the incoming request into the CGI environment
//! expected by `git http-backend` and stream its response back. Reads are
//! anonymous; writes require HTTP Basic auth where the password may be either
//! the account password (local convenience) or a personal access token.

use axum::{
    body::Bytes,
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode, Uri},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use loco_rs::prelude::*;
use serde::Deserialize;
use std::process::Stdio;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::data::settings::AppSettings;
use crate::models::{personal_access_tokens, repositories as repo_model, users};

fn ensure_git_repo_path(repo: &str) -> Result<()> {
    if !repo.ends_with(".git") {
        return not_found();
    }
    let bare = repo.strip_suffix(".git").unwrap_or(repo);
    repo_model::validate_name(bare)?;
    Ok(())
}

fn unauthorized_basic() -> Result<Response> {
    let mut response = (StatusCode::UNAUTHORIZED, "authentication required").into_response();
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static("Basic realm=\"Git\""),
    );
    Ok(response)
}

/// Authenticates a push. The Basic password is first tried as a PAT, then as
/// the account password. In both cases the resolved user must own the repo.
async fn verified_owner_login(
    ctx: &AppContext,
    headers: &HeaderMap,
    expected_owner: &str,
) -> Option<String> {
    if repo_model::validate_owner(expected_owner).is_err() {
        return None;
    }

    let auth_header = headers.get(header::AUTHORIZATION)?;
    let auth_str = auth_header.to_str().ok()?;
    let encoded = auth_str.strip_prefix("Basic ")?;
    let decoded_bytes = STANDARD.decode(encoded).ok()?;
    let credentials = String::from_utf8(decoded_bytes).ok()?;
    let (login, secret) = credentials.split_once(':')?;

    // A personal access token encodes its own row id, so it can be resolved
    // without trusting the Basic username.
    if let Ok((_token, user)) = personal_access_tokens::Entity::authenticate(&ctx.db, secret).await
    {
        if user.owner_handle() == expected_owner {
            return Some(user.owner_handle().to_owned());
        }
    }

    // Password fallback for local development. Accept either email or handle
    // as the Basic username.
    let user = match users::Model::find_by_email(&ctx.db, login).await {
        Ok(user) => user,
        Err(_) => users::Model::find_by_username(&ctx.db, login).await.ok()?,
    };
    if user.verify_password(secret) && user.owner_handle() == expected_owner {
        return Some(user.owner_handle().to_owned());
    }

    None
}

async fn run_git_http_backend(
    method: &str,
    git_project_root: &str,
    path_info: String,
    query: Option<&str>,
    content_type: Option<&str>,
    body: Bytes,
    remote_user: Option<&str>,
) -> Result<Response> {
    let mut cmd = Command::new("git");
    cmd.arg("http-backend")
        .env("GIT_PROJECT_ROOT", git_project_root)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("PATH_INFO", path_info)
        .env("REQUEST_METHOD", method)
        .env("QUERY_STRING", query.unwrap_or(""))
        .env("CONTENT_LENGTH", body.len().to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(ct) = content_type {
        cmd.env("CONTENT_TYPE", ct);
    }
    if let Some(user) = remote_user {
        cmd.env("REMOTE_USER", user);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::string(&format!("failed to spawn git http-backend: {e}")))?;

    if !body.is_empty() {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| Error::string("failed to open git http-backend stdin"))?;
        stdin
            .write_all(&body)
            .await
            .map_err(|e| Error::string(&format!("failed writing git request body: {e}")))?;
    }
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| Error::string(&format!("git http-backend execution failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::error!(stderr = %stderr, "git http-backend failed");
    }

    let stdout = output.stdout;
    let split = stdout
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| (i, 4))
        .or_else(|| stdout.windows(2).position(|w| w == b"\n\n").map(|i| (i, 2)))
        .ok_or_else(|| Error::string("invalid git http-backend response"))?;

    let headers_raw = &stdout[..split.0];
    let body_raw = stdout[split.0 + split.1..].to_vec();

    let mut status = StatusCode::OK;
    let mut response = Response::new(body_raw.into());

    for line in String::from_utf8_lossy(headers_raw).lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("Status:") {
            if let Some(code) = rest.split_whitespace().next() {
                if let Ok(code) = code.parse::<u16>() {
                    if let Ok(parsed) = StatusCode::from_u16(code) {
                        status = parsed;
                    }
                }
            }
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            let Ok(name) = HeaderName::from_bytes(name.trim().as_bytes()) else {
                continue;
            };
            let Ok(value) = HeaderValue::from_str(value.trim()) else {
                continue;
            };
            response.headers_mut().append(name, value);
        }
    }

    *response.status_mut() = status;
    Ok(response)
}

#[debug_handler]
async fn info_refs(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Query(query): Query<InfoRefsQuery>,
    uri: Uri,
    headers: HeaderMap,
) -> Result<Response> {
    let settings = AppSettings::from_context(&ctx)?;
    ensure_git_repo_path(&repo)?;

    // Pushes always need auth; private repositories also require it for reads.
    let needs_auth = matches!(query.service.as_deref(), Some("git-receive-pack"))
        || repo_is_private(&ctx, &owner, &repo).await;
    let remote_user = if needs_auth {
        let Some(login) = verified_owner_login(&ctx, &headers, &owner).await else {
            return unauthorized_basic();
        };
        Some(login)
    } else {
        None
    };

    run_git_http_backend(
        "GET",
        &settings.repo_base_dir,
        format!("/{owner}/{repo}/info/refs"),
        uri.query(),
        None,
        Bytes::new(),
        remote_user.as_deref(),
    )
    .await
}

#[debug_handler]
async fn upload_pack(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response> {
    let settings = AppSettings::from_context(&ctx)?;
    ensure_git_repo_path(&repo)?;

    let remote_user = if repo_is_private(&ctx, &owner, &repo).await {
        let Some(login) = verified_owner_login(&ctx, &headers, &owner).await else {
            return unauthorized_basic();
        };
        Some(login)
    } else {
        None
    };

    run_git_http_backend(
        "POST",
        &settings.repo_base_dir,
        format!("/{owner}/{repo}/git-upload-pack"),
        None,
        headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        body,
        remote_user.as_deref(),
    )
    .await
}

/// Whether a repository is private. An unknown repository fails closed so its
/// existence is not leaked through an unauthenticated response.
async fn repo_is_private(ctx: &AppContext, owner: &str, repo: &str) -> bool {
    let bare = repo.strip_suffix(".git").unwrap_or(repo);
    repo_model::Model::find_by_owner_and_name(&ctx.db, owner, bare)
        .await
        .map_or(true, |row| row.is_private())
}

/// Best-effort invalidation of the browse cache after a successful push.
/// Cache staleness must never fail an otherwise successful push.
async fn invalidate_repo_cache(ctx: &AppContext, owner: &str, repo: &str) {
    let bare = repo.strip_suffix(".git").unwrap_or(repo);
    if let Ok(row) =
        crate::models::repositories::Model::find_by_owner_and_name(&ctx.db, owner, bare).await
    {
        let _ = crate::models::branches::Entity::clear_for_repository(&ctx.db, row.id).await;
        let _ = crate::models::commits::Entity::clear_for_repository(&ctx.db, row.id).await;
    }
}

#[debug_handler]
async fn receive_pack(
    Path((owner, repo)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response> {
    let settings = AppSettings::from_context(&ctx)?;
    ensure_git_repo_path(&repo)?;

    let Some(login) = verified_owner_login(&ctx, &headers, &owner).await else {
        return unauthorized_basic();
    };

    let response = run_git_http_backend(
        "POST",
        &settings.repo_base_dir,
        format!("/{owner}/{repo}/git-receive-pack"),
        None,
        headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        body,
        Some(&login),
    )
    .await?;

    invalidate_repo_cache(&ctx, &owner, &repo).await;
    Ok(response)
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/{owner}/{repo}")
        .add("/info/refs", get(info_refs))
        .add("/git-upload-pack", post(upload_pack))
        .add("/git-receive-pack", post(receive_pack))
}

#[derive(Debug, Deserialize)]
struct InfoRefsQuery {
    service: Option<String>,
}
