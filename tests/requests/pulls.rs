//! Request tests for the pull request flow against a real bare repository.

use axum::http::StatusCode;
use github_loco::{
    app::App,
    dtos::pulls::{CompareResult, PullRequestResponse, ReviewCommentResponse},
};
use loco_rs::testing::prelude::*;
use serde_json::{json, Value};
use serial_test::serial;

use super::{prepare_data, support};

/// Registers a user, creates a public repo, and seeds `main` plus a `feature`
/// branch containing one new file.
async fn setup_repo_with_feature(
    request: &loco_rs::TestServer,
    ctx: &loco_rs::app::AppContext,
    root: &support::TestRepoRoot,
    private: bool,
) -> (axum::http::HeaderName, axum::http::HeaderValue) {
    let logged = prepare_data::init_user_login(request, ctx).await;
    let (key, value) = prepare_data::auth_header(&logged.token);
    let created = request
        .post("/api/repositories")
        .add_header(key.clone(), value.clone())
        .json(&json!({ "name": "hello", "is_private": private }))
        .await;
    assert_eq!(created.status_code(), 200, "{}", created.text());

    let bare = root.bare_repo("loco", "hello");
    support::seed_commit(&bare);
    support::seed_feature_branch(&bare, "feature", "feature.txt", "new feature\n");
    (key, value)
}

fn create_pr_body() -> Value {
    json!({
        "title": "Add the feature",
        "body": "Please review",
        "head_ref": "feature",
        "base_ref": "main"
    })
}

#[tokio::test]
#[serial]
async fn can_open_compare_comment_and_merge_a_pull_request() {
    let root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let (key, value) = setup_repo_with_feature(&request, &ctx, &root, false).await;

        let created = request
            .post("/api/repos/loco/hello/pulls")
            .add_header(key.clone(), value.clone())
            .json(&create_pr_body())
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());
        let pr: PullRequestResponse = serde_json::from_str(&created.text()).unwrap();
        assert_eq!(pr.number, 1);
        assert_eq!(pr.state, "open");
        assert_eq!(pr.author, "loco");
        assert_eq!(pr.head_ref, "feature");
        assert_eq!(pr.base_ref, "main");
        assert!(!pr.head_sha.is_empty());

        // A second PR gets the next number for this repository.
        let second = request
            .post("/api/repos/loco/hello/pulls")
            .add_header(key.clone(), value.clone())
            .json(&create_pr_body())
            .await;
        assert_eq!(second.status_code(), 200, "{}", second.text());
        let second: PullRequestResponse = serde_json::from_str(&second.text()).unwrap();
        assert_eq!(second.number, 2);

        let listed = request.get("/api/repos/loco/hello/pulls").await;
        assert_eq!(listed.status_code(), 200);
        let prs: Vec<PullRequestResponse> = serde_json::from_str(&listed.text()).unwrap();
        assert_eq!(prs.len(), 2);

        let open = request.get("/api/repos/loco/hello/pulls?state=open").await;
        let open: Vec<PullRequestResponse> = serde_json::from_str(&open.text()).unwrap();
        assert_eq!(open.len(), 2);

        let detail = request.get("/api/repos/loco/hello/pulls/1").await;
        assert_eq!(detail.status_code(), 200);
        let detail: PullRequestResponse = serde_json::from_str(&detail.text()).unwrap();
        assert_eq!(detail.title, "Add the feature");

        // Three-dot compare: one commit ahead, one new file with a patch.
        let compare = request
            .get("/api/repos/loco/hello/pulls/compare?head=feature&base=main")
            .await;
        assert_eq!(compare.status_code(), 200, "{}", compare.text());
        let compare: CompareResult = serde_json::from_str(&compare.text()).unwrap();
        assert_eq!(compare.ahead_by, 1);
        assert_eq!(compare.behind_by, 0);
        assert_eq!(compare.commits.len(), 1);
        assert_eq!(compare.files.len(), 1);
        assert_eq!(compare.files[0].path, "feature.txt");
        assert_eq!(compare.files[0].status, "added");
        assert_eq!(compare.files[0].additions, 1);
        assert!(compare.files[0].patch.contains("+new feature"));

        // Review comments, optionally anchored to a line.
        let commented = request
            .post("/api/repos/loco/hello/pulls/1/comments")
            .add_header(key.clone(), value.clone())
            .json(&json!({
                "body": "Looks good",
                "path": "feature.txt",
                "line": 1
            }))
            .await;
        assert_eq!(commented.status_code(), 200, "{}", commented.text());
        let comment: ReviewCommentResponse = serde_json::from_str(&commented.text()).unwrap();
        assert_eq!(comment.author, "loco");
        assert_eq!(comment.path.as_deref(), Some("feature.txt"));
        assert_eq!(comment.line, Some(1));

        let comments = request.get("/api/repos/loco/hello/pulls/1/comments").await;
        let comments: Vec<ReviewCommentResponse> = serde_json::from_str(&comments.text()).unwrap();
        assert_eq!(comments.len(), 1);

        // Merging runs in a worker (inline in test mode) and updates the PR.
        let merged = request
            .post("/api/repos/loco/hello/pulls/1/merge")
            .add_header(key.clone(), value.clone())
            .await;
        assert_eq!(merged.status_code(), 200, "{}", merged.text());
        let merged: PullRequestResponse = serde_json::from_str(&merged.text()).unwrap();
        assert_eq!(merged.state, "merged");
        assert!(merged.merged_at.is_some());
        assert!(!merged.merged_commit_sha.unwrap().is_empty());

        // Git itself now has the feature file on main.
        let raw = request
            .get("/api/repos/loco/hello/raw?ref=main&path=feature.txt")
            .await;
        assert_eq!(raw.status_code(), 200, "{}", raw.text());
        assert_eq!(raw.text(), "new feature\n");

        let merged_list = request
            .get("/api/repos/loco/hello/pulls?state=merged")
            .await;
        let merged_list: Vec<PullRequestResponse> =
            serde_json::from_str(&merged_list.text()).unwrap();
        assert_eq!(merged_list.len(), 1);

        // A merged PR cannot be reopened.
        let reopen = request
            .post("/api/repos/loco/hello/pulls/1/reopen")
            .add_header(key, value)
            .await;
        assert_eq!(reopen.status_code(), StatusCode::BAD_REQUEST);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn private_pull_requests_need_auth_and_merge_requires_a_token() {
    let root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let (key, value) = setup_repo_with_feature(&request, &ctx, &root, true).await;

        let created = request
            .post("/api/repos/loco/hello/pulls")
            .add_header(key.clone(), value.clone())
            .json(&create_pr_body())
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());

        for path in [
            "/api/repos/loco/hello/pulls",
            "/api/repos/loco/hello/pulls/1",
            "/api/repos/loco/hello/pulls/compare?head=feature&base=main",
        ] {
            let anonymous = request.get(path).await;
            assert_eq!(anonymous.status_code(), 404, "anonymous read of {path}");
        }

        let allowed = request
            .get("/api/repos/loco/hello/pulls")
            .add_header(key.clone(), value)
            .await;
        assert_eq!(allowed.status_code(), 200, "{}", allowed.text());

        let anonymous_merge = request.post("/api/repos/loco/hello/pulls/1/merge").await;
        assert_eq!(anonymous_merge.status_code(), StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn a_conflicting_merge_leaves_the_pull_request_open() {
    let root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let (key, value) = setup_repo_with_feature(&request, &ctx, &root, false).await;

        // Both branches change README.md from the same base, so the merge has
        // to report a conflict rather than silently overwriting a side.
        let bare = root.bare_repo("loco", "hello");
        support::seed_feature_branch(&bare, "conflict", "README.md", "# from the head\n");
        support::seed_branch_commit(&bare, "main", "README.md", "# changed on main\n", false);

        let created = request
            .post("/api/repos/loco/hello/pulls")
            .add_header(key.clone(), value.clone())
            .json(&json!({
                "title": "Conflicting change",
                "body": null,
                "head_ref": "conflict",
                "base_ref": "main"
            }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());

        let merged = request
            .post("/api/repos/loco/hello/pulls/1/merge")
            .add_header(key, value)
            .await;
        assert_eq!(
            merged.status_code(),
            StatusCode::BAD_REQUEST,
            "{}",
            merged.text()
        );
        assert!(merged.text().contains("conflict"));

        let detail = request.get("/api/repos/loco/hello/pulls/1").await;
        let detail: PullRequestResponse = serde_json::from_str(&detail.text()).unwrap();
        assert_eq!(detail.state, "open");
        assert!(detail.merged_commit_sha.is_none());
    })
    .await;
}
