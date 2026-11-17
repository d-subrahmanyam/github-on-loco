//! Request tests for the read-only git browsing API.

use github_loco::{
    app::App,
    dtos::repositories::{Branch, Commit, CommitDetail, RepoResponse, TreeEntry},
    models::{branches, commits},
};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

use super::{prepare_data, support};

async fn create_repo_with_commit(request: &loco_rs::TestServer, ctx: &loco_rs::app::AppContext) {
    let logged = prepare_data::init_user_login(request, ctx).await;
    let (key, value) = prepare_data::auth_header(&logged.token);

    let created = request
        .post("/api/repositories")
        .add_header(key, value)
        .json(&json!({ "name": "hello", "is_private": false }))
        .await;
    assert_eq!(created.status_code(), 200, "{}", created.text());
}

#[tokio::test]
#[serial]
async fn can_browse_branches_commits_tree_and_blobs() {
    let root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        create_repo_with_commit(&request, &ctx).await;
        support::seed_commit(&root.bare_repo("loco", "hello"));

        let overview = request.get("/api/repos/loco/hello").await;
        assert_eq!(overview.status_code(), 200, "{}", overview.text());
        let repo: RepoResponse = serde_json::from_str(&overview.text()).unwrap();

        let branches = request.get("/api/repos/loco/hello/branches").await;
        assert_eq!(branches.status_code(), 200, "{}", branches.text());
        let branch_list: Vec<Branch> = serde_json::from_str(&branches.text()).unwrap();
        assert!(branch_list.iter().any(|b| b.name == "main"));
        assert!(
            !branches::Entity::find_by_repository(&ctx.db, repo.id)
                .await
                .unwrap()
                .is_empty(),
            "browsing should warm the branch cache"
        );

        let commits = request.get("/api/repos/loco/hello/commits").await;
        assert_eq!(commits.status_code(), 200, "{}", commits.text());
        let commit_list: Vec<Commit> = serde_json::from_str(&commits.text()).unwrap();
        assert_eq!(commit_list.len(), 1);
        assert_eq!(commit_list[0].message, "initial commit");
        assert!(
            !commits::Entity::find_by_repository(&ctx.db, repo.id)
                .await
                .unwrap()
                .is_empty(),
            "browsing should warm the commit cache"
        );

        let detail = request
            .get(&format!(
                "/api/repos/loco/hello/commit/{}",
                commit_list[0].sha
            ))
            .await;
        assert_eq!(detail.status_code(), 200, "{}", detail.text());
        let detail: CommitDetail = serde_json::from_str(&detail.text()).unwrap();
        assert_eq!(detail.message, "initial commit");

        let tree = request.get("/api/repos/loco/hello/tree").await;
        assert_eq!(tree.status_code(), 200, "{}", tree.text());
        let tree: Vec<TreeEntry> = serde_json::from_str(&tree.text()).unwrap();
        assert!(tree.iter().any(|entry| entry.name == "README.md"));

        let raw = request
            .get("/api/repos/loco/hello/raw?ref=main&path=README.md")
            .await;
        assert_eq!(raw.status_code(), 200, "{}", raw.text());
        assert!(raw.text().contains("# hello"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn rejects_unsafe_revision_names() {
    let root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        create_repo_with_commit(&request, &ctx).await;
        support::seed_commit(&root.bare_repo("loco", "hello"));

        let response = request.get("/api/repos/loco/hello/commits?ref=--all").await;
        assert_eq!(response.status_code(), 400);

        let response = request
            .get("/api/repos/loco/hello/tree?ref=main&path=../secret")
            .await;
        assert_eq!(response.status_code(), 400);
    })
    .await;
}
