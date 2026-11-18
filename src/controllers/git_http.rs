//! Git smart-HTTP transport for clone, fetch and push.
//!
//! Loco handles REST; git has its own wire protocol. Rather than reimplement
//! it, these handlers translate the incoming request into the CGI environment
//! expected by `git http-backend` and stream its response back. Public reads
//! are anonymous; private reads and all writes require HTTP Basic auth, with
//! the same `repositories::Model::can` rule used by the REST handlers. The
//! password may be a personal access token or (locally) the account password.

use axum::{
    body::Bytes,
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode, Uri},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use loco_rs::prelude::*;
use serde::Deserialize;
use serde_json::json;
use std::process::Stdio;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::data::{git::GitRepo, settings::AppSettings};
use crate::models::{
    branch_protection_rules as protection, events, personal_access_tokens, pull_requests,
    repositories as repo_model, users,
};

fn ensure_git_repo_path(repo: &str) -> Result<()> {
    if !repo.ends_with(".git") {
        return not_found();
    }
    let bare = repo.strip_suffix(".git").unwrap_or(repo);
    repo_model::validate_name(bare)?;
    Ok(())
}

/// No credentials (or ones that do not resolve to a user). The
/// `WWW-Authenticate` header makes git prompt for credentials again.
fn unauthorized_basic_response() -> Response {
    let mut response = (StatusCode::UNAUTHORIZED, "authentication required").into_response();
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static("Basic realm=\"Git\""),
    );
    response
}

/// The path has no repository. An authenticated caller gets a clear answer;
/// anonymous callers keep the opaque 401 so a missing repo cannot be told apart
/// from a private one.
fn repository_not_found_response() -> Response {
    (StatusCode::NOT_FOUND, "repository not found").into_response()
}

/// Credentials are valid but lack the required action. There is no
/// `WWW-Authenticate` header: re-prompting with the same credentials would not
/// help, so git reports the reason instead of looping.
fn permission_denied_response() -> Response {
    (
        StatusCode::FORBIDDEN,
        "permission denied: write access to this repository is required",
    )
        .into_response()
}

/// Resolves Basic credentials to a user. The password is first tried as a PAT
/// (which encodes its own row id, so the username is not trusted), then as the
/// account password for local development.
async fn authenticated_user(ctx: &AppContext, headers: &HeaderMap) -> Option<users::Model> {
    let auth_str = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let encoded = auth_str.strip_prefix("Basic ")?;
    let credentials = String::from_utf8(STANDARD.decode(encoded).ok()?).ok()?;
    let (login, secret) = credentials.split_once(':')?;

    if let Ok((_token, user)) = personal_access_tokens::Entity::authenticate(&ctx.db, secret).await
    {
        return Some(user);
    }

    // Accept either email or handle as the Basic username.
    match users::Model::find_by_email(&ctx.db, login).await {
        Ok(user) if user.verify_password(secret) => Some(user),
        _ => users::Model::find_by_username(&ctx.db, login)
            .await
            .ok()
            .filter(|user| user.verify_password(secret)),
    }
}

/// Result of resolving a git request: the caller (if auth was needed) and the
/// repository, or the HTTP response that should be sent instead.
type GitAuthResult = std::result::Result<(Option<users::Model>, repo_model::Model), Box<Response>>;

/// Authenticates a git request through the repository's single `can` rule.
///
/// The three failure cases are kept distinct: missing credentials are a 401,
/// a missing repo is a 404 for authenticated callers, and valid credentials
/// without permission are a 403 — except on private repos, which answer 404 so
/// their existence is never leaked (matching the REST endpoints).
async fn authorize_git(
    ctx: &AppContext,
    headers: &HeaderMap,
    owner: &str,
    repo: &str,
    action: repo_model::RepoAction,
) -> GitAuthResult {
    let Some(row) = find_git_repo(ctx, owner, repo).await else {
        return Err(Box::new(
            if authenticated_user(ctx, headers).await.is_some() {
                repository_not_found_response()
            } else {
                unauthorized_basic_response()
            },
        ));
    };

    // Public reads are anonymous; there is no caller to attribute the request to.
    if action == repo_model::RepoAction::Read && !row.is_private() {
        return Ok((None, row));
    }

    let Some(user) = authenticated_user(ctx, headers).await else {
        return Err(Box::new(unauthorized_basic_response()));
    };

    match row.can(&ctx.db, Some(user.id), action).await {
        Ok(true) => Ok((Some(user), row)),
        Ok(false) => Err(Box::new(if row.is_private() {
            repository_not_found_response()
        } else {
            permission_denied_response()
        })),
        Err(err) => {
            tracing::error!(%err, "git authorization lookup failed");
            Err(Box::new(unauthorized_basic_response()))
        }
    }
}

/// A protected branch refuses direct pushes. The merge worker writes via git
/// plumbing instead, so pull requests remain the only way in.
fn protected_branch_rejected() -> Result<Response> {
    Ok((
        StatusCode::FORBIDDEN,
        "branch is protected: merge through a pull request",
    )
        .into_response())
}

/// Extracts the branch name from the first pkt-line of a receive-pack request:
/// `<old-sha> <new-sha> refs/heads/<branch>\0<capabilities>`.
fn pushed_ref(body: &Bytes) -> Option<String> {
    let text = String::from_utf8_lossy(body);
    let len = usize::from_str_radix(text.get(0..4)?, 16).ok()?;
    let payload = text.get(4..4 + len)?;
    let mut parts = payload.split_whitespace();
    let _old = parts.next()?;
    let _new = parts.next()?;
    let reference = parts.next()?.split('\0').next()?;
    Some(
        reference
            .strip_prefix("refs/heads/")
            .unwrap_or(reference)
            .to_owned(),
    )
}

/// Loads a repository from the `.git`-suffixed path segment.
async fn find_git_repo(ctx: &AppContext, owner: &str, repo: &str) -> Option<repo_model::Model> {
    let bare = repo.strip_suffix(".git").unwrap_or(repo);
    repo_model::Model::find_by_owner_and_name(&ctx.db, owner, bare)
        .await
        .ok()
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
    let is_push = matches!(query.service.as_deref(), Some("git-receive-pack"));
    let action = if is_push {
        repo_model::RepoAction::Write
    } else {
        repo_model::RepoAction::Read
    };
    let (user, _row) = match authorize_git(&ctx, &headers, &owner, &repo, action).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let remote_user = user.map(|user| user.owner_handle().to_owned());

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

    let (user, _row) =
        match authorize_git(&ctx, &headers, &owner, &repo, repo_model::RepoAction::Read).await {
            Ok(pair) => pair,
            Err(response) => return Ok(*response),
        };
    let remote_user = user.map(|user| user.owner_handle().to_owned());

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

/// Best-effort invalidation of the browse cache after a successful push.
/// Cache staleness must never fail an otherwise successful push.
async fn invalidate_repo_cache(ctx: &AppContext, owner: &str, repo: &str) {
    if let Some(row) = find_git_repo(ctx, owner, repo).await {
        let _ = crate::models::branches::Entity::clear_for_repository(&ctx.db, row.id).await;
        let _ = crate::models::commits::Entity::clear_for_repository(&ctx.db, row.id).await;
    }
}

/// Refreshes the recorded head SHA of open PRs after a push.
///
/// Git stays authoritative: we re-resolve each PR's head ref rather than
/// parsing the receive-pack payload. A push that did not move a ref is a no-op.
/// Failures are logged and swallowed — a PR bookkeeping issue must never turn
/// an otherwise successful push into an error. Public so request tests can
/// exercise the same post-push bookkeeping against a local bare repository.
pub async fn sync_pull_request_heads(ctx: &AppContext, owner: &str, repo: &str) {
    let bare = repo.strip_suffix(".git").unwrap_or(repo);
    let Some(row) = find_git_repo(ctx, owner, repo).await else {
        return;
    };
    let Ok(prs) =
        pull_requests::Model::find_by_repository(&ctx.db, row.id, Some(pull_requests::STATE_OPEN))
            .await
    else {
        return;
    };
    if prs.is_empty() {
        return;
    }

    let Ok(settings) = AppSettings::from_context(ctx) else {
        return;
    };
    let Ok(git) = GitRepo::open(&settings.repo_base_dir, owner, bare) else {
        return;
    };
    for pr in prs {
        let Ok(sha) = git.resolve(&pr.head_ref).await else {
            continue;
        };
        if sha == pr.head_sha {
            continue;
        }
        let active = pull_requests::ActiveModel {
            id: Set(pr.id),
            ..Default::default()
        };
        if let Err(err) = active.set_head_sha(&ctx.db, &sha).await {
            tracing::warn!(pull_request_id = pr.id, %err, "failed to refresh PR head SHA");
        }
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

    let (caller, row) =
        match authorize_git(&ctx, &headers, &owner, &repo, repo_model::RepoAction::Write).await {
            Ok(pair) => pair,
            Err(response) => return Ok(*response),
        };
    // Pushes always authenticate, so a caller is guaranteed here.
    let Some(user) = caller else {
        return Ok(unauthorized_basic_response());
    };
    let login = user.owner_handle().to_owned();

    let branch = pushed_ref(&body).unwrap_or_else(|| "a branch".to_owned());
    // Admins (including the owner) stay exempt, matching GitHub's default.
    if let Some(rule) = protection::Entity::find_for_branch(&ctx.db, row.id, &branch).await? {
        if rule.require_pull_request() && !row.can_admin(&ctx.db, user.id).await? {
            return protected_branch_rejected();
        }
    }
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

    // Best-effort activity record; a failed event must not fail a push.
    if let Ok(actor) = users::Model::find_by_username(&ctx.db, &login).await {
        let payload = json!({ "branch": branch });
        let _ = crate::services::events::record(
            &ctx,
            row.id,
            actor.id,
            events::kind::PUSH,
            Some(payload),
        )
        .await;
    }
    invalidate_repo_cache(&ctx, &owner, &repo).await;
    sync_pull_request_heads(&ctx, &owner, &repo).await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_the_branch_from_a_receive_pack_body() {
        let payload = format!(
            "{} {} refs/heads/feature/x\0report-status",
            "0".repeat(40),
            "1".repeat(40)
        );
        let body = Bytes::from(format!("{:04x}{payload}", payload.len()));
        assert_eq!(pushed_ref(&body).as_deref(), Some("feature/x"));
    }
}

#[derive(Debug, Deserialize)]
struct InfoRefsQuery {
    service: Option<String>,
}
