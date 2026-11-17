//! Request tests for repository creation and listing.

use github_loco::{app::App, dtos::repositories::RepoResponse};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

use super::{prepare_data, support::TestRepoRoot};

#[tokio::test]
#[serial]
async fn can_create_and_list_repositories() {
    let root = TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let logged = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&logged.token);

        let created = request
            .post("/api/repositories")
            .add_header(key.clone(), value.clone())
            .json(&json!({
                "name": "hello",
                "description": "first repo",
                "is_private": false
            }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());

        let repo: RepoResponse =
            serde_json::from_str(&created.text()).expect("response should be a RepoResponse");
        assert_eq!(repo.name, "hello");
        assert_eq!(
            repo.owner, "loco",
            "owner must be the username, not the display name"
        );
        assert!(!repo.is_private);
        assert!(repo.clone_url.ends_with("/loco/hello.git"));

        // The DB row and the bare repository must both exist.
        assert!(root.bare_repo("loco", "hello").join("HEAD").is_file());

        let listed = request
            .get("/api/repositories")
            .add_header(key, value)
            .await;
        assert_eq!(listed.status_code(), 200);
        let repos: Vec<RepoResponse> =
            serde_json::from_str(&listed.text()).expect("response should be a list");
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "hello");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn rejects_unsafe_repository_names() {
    let _root = TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let logged = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&logged.token);

        for name in ["../escape", "a/b", "with space", "repo.git"] {
            let response = request
                .post("/api/repositories")
                .add_header(key.clone(), value.clone())
                .json(&json!({ "name": name, "is_private": true }))
                .await;
            assert_eq!(
                response.status_code(),
                400,
                "name {name:?} should be rejected"
            );
        }
    })
    .await;
}
