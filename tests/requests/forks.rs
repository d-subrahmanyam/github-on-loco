//! Request tests for forking: git copy, metadata, visibility and naming.

use github_loco::{app::App, dtos::forks::ForkResponse, dtos::repositories::RepoResponse};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;
use std::process::Command;

use super::{prepare_data, support};

async fn create_repo(
    request: &loco_rs::TestServer,
    key: &axum::http::HeaderName,
    value: &axum::http::HeaderValue,
    name: &str,
    is_private: bool,
) {
    let response = request
        .post("/api/repositories")
        .add_header(key.clone(), value.clone())
        .json(&json!({ "name": name, "is_private": is_private }))
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
}

#[tokio::test]
#[serial]
async fn forking_copies_history_and_records_the_relationship() {
    let root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (owner_key, owner_value) = prepare_data::auth_header(&owner.token);
        create_repo(&request, &owner_key, &owner_value, "project", false).await;

        let bare = root.bare_repo("loco", "project");
        support::seed_commit(&bare);
        support::seed_feature_branch(&bare, "feature", "feature.txt", "feature\n");

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

        let created = request
            .post("/api/repos/loco/project/fork")
            .add_header(key.clone(), value.clone())
            .json(&json!({ "name": null }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());
        let fork: RepoResponse = serde_json::from_str(&created.text()).unwrap();
        assert_eq!(fork.owner, "guest");
        assert_eq!(fork.name, "project");
        assert_eq!(fork.viewer_role, "owner");
        let parent = fork.parent.expect("a fork has a parent");
        assert_eq!(parent.owner, "loco");
        assert_eq!(parent.name, "project");

        // The fork is a full bare copy: both branches came across.
        let fork_bare = root.bare_repo("guest", "project");
        assert!(fork_bare.join("HEAD").is_file());
        for branch in ["main", "feature"] {
            let output = Command::new("git")
                .arg("-C")
                .arg(&fork_bare)
                .args(["rev-parse", branch])
                .output()
                .unwrap();
            assert!(output.status.success(), "branch {branch} was not copied");
        }

        // Both sides see the relationship.
        let source = request.get("/api/repos/loco/project").await;
        let source: RepoResponse = serde_json::from_str(&source.text()).unwrap();
        assert_eq!(source.fork_count, 1);
        assert!(source.parent.is_none());

        let fork_view = request.get("/api/repos/guest/project").await;
        let fork_view: RepoResponse = serde_json::from_str(&fork_view.text()).unwrap();
        assert_eq!(fork_view.parent.unwrap().owner, "loco");

        let list = request.get("/api/repos/loco/project/forks").await;
        let forks: Vec<ForkResponse> = serde_json::from_str(&list.text()).unwrap();
        assert_eq!(forks.len(), 1);
        assert_eq!(forks[0].owner, "guest");
        assert_eq!(forks[0].name, "project");

        // A second fork gets a free name instead of colliding.
        let second = request
            .post("/api/repos/loco/project/fork")
            .add_header(key.clone(), value.clone())
            .json(&json!({}))
            .await;
        assert_eq!(second.status_code(), 200, "{}", second.text());
        let second: RepoResponse = serde_json::from_str(&second.text()).unwrap();
        assert_eq!(second.name, "project-fork-1");

        // Forking your own repo is pointless and rejected.
        let own = request
            .post("/api/repos/loco/project/fork")
            .add_header(owner_key, owner_value)
            .json(&json!({}))
            .await;
        assert_eq!(own.status_code(), 400, "{}", own.text());

        let feed = request.get("/api/repos/loco/project/events").await;
        assert!(feed.text().contains("\"kind\":\"fork\""), "{}", feed.text());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn forking_respects_authentication_and_visibility() {
    let _root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (owner_key, owner_value) = prepare_data::auth_header(&owner.token);
        create_repo(&request, &owner_key, &owner_value, "hidden", true).await;

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

        let anonymous = request
            .post("/api/repos/loco/hidden/fork")
            .json(&json!({}))
            .await;
        assert_eq!(anonymous.status_code(), 401);

        let hidden = request
            .post("/api/repos/loco/hidden/fork")
            .add_header(key.clone(), value.clone())
            .json(&json!({}))
            .await;
        assert_eq!(hidden.status_code(), 404, "{}", hidden.text());

        // A public repository can be forked by any signed-in user.
        create_repo(&request, &owner_key, &owner_value, "open", false).await;
        let public = request
            .post("/api/repos/loco/open/fork")
            .add_header(key, value)
            .json(&json!({}))
            .await;
        assert_eq!(public.status_code(), 200, "{}", public.text());
    })
    .await;
}
