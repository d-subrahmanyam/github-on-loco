//! Request tests for branch protection and PR approvals.

use axum::body::Bytes;
use axum::http::{HeaderName, HeaderValue};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use github_loco::{
    app::App,
    dtos::protection::{BranchProtectionResponse, PrReviewResponse},
    dtos::pulls::PullRequestResponse,
};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

use super::{prepare_data, support};

fn basic(login: &str, password: &str) -> (HeaderName, HeaderValue) {
    let encoded = STANDARD.encode(format!("{login}:{password}"));
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Basic {encoded}")).unwrap(),
    )
}

/// A minimal receive-pack pkt-line: `<old> <new> refs/heads/<branch>\0caps`.
fn push_body(branch: &str) -> Bytes {
    let payload = format!(
        "{} {} refs/heads/{branch}\0report-status",
        "0".repeat(40),
        "1".repeat(40)
    );
    Bytes::from(format!("{:04x}{payload}", payload.len()))
}

async fn create_repo(
    request: &loco_rs::TestServer,
    key: &HeaderName,
    value: &HeaderValue,
    name: &str,
) {
    let response = request
        .post("/api/repositories")
        .add_header(key.clone(), value.clone())
        .json(&json!({ "name": name, "is_private": false }))
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
}

#[tokio::test]
#[serial]
async fn protection_settings_are_admin_only() {
    let _root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (owner_key, owner_value) = prepare_data::auth_header(&owner.token);
        create_repo(&request, &owner_key, &owner_value, "settings").await;

        let guest = prepare_data::register_and_login(
            &request,
            &ctx,
            "guest@example.com",
            "Guest",
            Some("guest"),
            "secret",
        )
        .await;
        let (key, value) = prepare_data::auth_header(&guest.token);
        let base = "/api/repos/loco/settings/branch-protection";

        for response in [
            request
                .get(base)
                .add_header(key.clone(), value.clone())
                .await,
            request
                .post(base)
                .add_header(key.clone(), value.clone())
                .json(&json!({
                    "branch": "main",
                    "require_pull_request": true,
                    "required_approvals": 1
                }))
                .await,
            request
                .delete(&format!("{base}?branch=main"))
                .add_header(key.clone(), value.clone())
                .await,
        ] {
            assert_eq!(response.status_code(), 404, "{}", response.text());
        }

        let created = request
            .post(base)
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({
                "branch": "main",
                "require_pull_request": true,
                "required_approvals": 2
            }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());
        let rule: BranchProtectionResponse = serde_json::from_str(&created.text()).unwrap();
        assert_eq!(rule.branch, "main");
        assert!(rule.require_pull_request);
        assert_eq!(rule.required_approvals, 2);

        // Granting admin makes the same endpoints visible to the guest.
        let granted = request
            .post("/api/repos/loco/settings/collaborators")
            .add_header(owner_key, owner_value)
            .json(&json!({ "username": "guest", "role": "admin" }))
            .await;
        assert_eq!(granted.status_code(), 200, "{}", granted.text());

        let list = request.get(base).add_header(key, value).await;
        assert_eq!(list.status_code(), 200, "{}", list.text());
        let rules: Vec<BranchProtectionResponse> = serde_json::from_str(&list.text()).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].branch, "main");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn merges_wait_for_non_author_approvals() {
    let root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (owner_key, owner_value) = prepare_data::auth_header(&owner.token);
        create_repo(&request, &owner_key, &owner_value, "approval").await;

        let bare = root.bare_repo("loco", "approval");
        support::seed_commit(&bare);
        support::seed_feature_branch(&bare, "feature", "feature.txt", "feature\n");

        let rule = request
            .post("/api/repos/loco/approval/branch-protection")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({
                "branch": "main",
                "require_pull_request": true,
                "required_approvals": 1
            }))
            .await;
        assert_eq!(rule.status_code(), 200, "{}", rule.text());

        let created = request
            .post("/api/repos/loco/approval/pulls")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({
                "title": "Feature",
                "body": null,
                "head_ref": "feature",
                "base_ref": "main"
            }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());

        let self_review = request
            .post("/api/repos/loco/approval/pulls/1/reviews")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({ "state": "approved", "body": null }))
            .await;
        assert_eq!(self_review.status_code(), 400, "{}", self_review.text());

        let blocked = request
            .post("/api/repos/loco/approval/pulls/1/merge")
            .add_header(owner_key.clone(), owner_value.clone())
            .await;
        assert_eq!(blocked.status_code(), 400, "{}", blocked.text());
        assert!(blocked.text().contains("approving review"));

        let detail = request
            .get("/api/repos/loco/approval/pulls/1")
            .add_header(owner_key.clone(), owner_value.clone())
            .await;
        let pr: PullRequestResponse = serde_json::from_str(&detail.text()).unwrap();
        assert_eq!(pr.required_approvals, 1);
        assert_eq!(pr.approval_count, 0);
        assert!(pr.merge_blocked_reason.is_some());

        let guest = prepare_data::register_and_login(
            &request,
            &ctx,
            "reviewer@example.com",
            "Reviewer",
            Some("reviewer"),
            "secret",
        )
        .await;
        let (key, value) = prepare_data::auth_header(&guest.token);
        let granted = request
            .post("/api/repos/loco/approval/collaborators")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({ "username": "reviewer", "role": "write" }))
            .await;
        assert_eq!(granted.status_code(), 200, "{}", granted.text());

        let reviewed = request
            .post("/api/repos/loco/approval/pulls/1/reviews")
            .add_header(key.clone(), value.clone())
            .json(&json!({ "state": "approved", "body": "looks good" }))
            .await;
        assert_eq!(reviewed.status_code(), 200, "{}", reviewed.text());
        let review: PrReviewResponse = serde_json::from_str(&reviewed.text()).unwrap();
        assert_eq!(review.state, "approved");
        assert_eq!(review.author, "reviewer");

        let reviews = request
            .get("/api/repos/loco/approval/pulls/1/reviews")
            .await;
        let all: Vec<PrReviewResponse> = serde_json::from_str(&reviews.text()).unwrap();
        assert_eq!(all.len(), 1);

        let detail = request.get("/api/repos/loco/approval/pulls/1").await;
        let pr: PullRequestResponse = serde_json::from_str(&detail.text()).unwrap();
        assert_eq!(pr.approval_count, 1);
        assert!(pr.merge_blocked_reason.is_none());

        let merged = request
            .post("/api/repos/loco/approval/pulls/1/merge")
            .add_header(owner_key.clone(), owner_value.clone())
            .await;
        assert_eq!(merged.status_code(), 200, "{}", merged.text());
        let pr: PullRequestResponse = serde_json::from_str(&merged.text()).unwrap();
        assert_eq!(pr.state, "merged");

        let feed = request.get("/api/repos/loco/approval/events").await;
        assert!(feed.text().contains("pr.approved"), "{}", feed.text());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn protected_branches_reject_direct_pushes_except_for_admins() {
    let _root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (owner_key, owner_value) = prepare_data::auth_header(&owner.token);
        create_repo(&request, &owner_key, &owner_value, "guarded").await;
        let bare = _root.bare_repo("loco", "guarded");
        support::seed_commit(&bare);

        let rule = request
            .post("/api/repos/loco/guarded/branch-protection")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({
                "branch": "main",
                "require_pull_request": true,
                "required_approvals": 0
            }))
            .await;
        assert_eq!(rule.status_code(), 200, "{}", rule.text());

        let _guest = prepare_data::register_and_login(
            &request,
            &ctx,
            "writer@example.com",
            "Writer",
            Some("writer"),
            "secret",
        )
        .await;
        let granted = request
            .post("/api/repos/loco/guarded/collaborators")
            .add_header(owner_key, owner_value)
            .json(&json!({ "username": "writer", "role": "write" }))
            .await;
        assert_eq!(granted.status_code(), 200, "{}", granted.text());

        let url = "/loco/guarded.git/git-receive-pack";
        let writer_auth = basic("writer", "secret");
        let rejected = request
            .post(url)
            .add_header(writer_auth.0, writer_auth.1)
            .bytes(push_body("main"))
            .await;
        assert_eq!(rejected.status_code(), 403, "{}", rejected.text());
        assert!(rejected.text().contains("protected"));

        // The owner is an admin and stays exempt, so the request reaches git
        // (which may reject the synthetic body) rather than the rule check.
        let owner_auth = basic("loco", "1234");
        let allowed = request
            .post(url)
            .add_header(owner_auth.0, owner_auth.1)
            .bytes(push_body("main"))
            .await;
        assert_ne!(allowed.status_code(), 403, "{}", allowed.text());
    })
    .await;
}
