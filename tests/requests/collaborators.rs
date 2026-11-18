//! Request tests for collaborator management and its effect on git auth.

use axum::http::{HeaderName, HeaderValue};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use github_loco::{app::App, dtos::collaborators::CollaboratorResponse};
use loco_rs::testing::prelude::*;
use serde_json::{json, Value};
use serial_test::serial;

use super::{prepare_data, support};

fn basic_header(username: &str, password: &str) -> (HeaderName, HeaderValue) {
    let encoded = STANDARD.encode(format!("{username}:{password}"));
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Basic {encoded}")).unwrap(),
    )
}

async fn create_private_repo(request: &loco_rs::TestServer, key: &HeaderName, value: &HeaderValue) {
    let response = request
        .post("/api/repositories")
        .add_header(key.clone(), value.clone())
        .json(&json!({ "name": "secret", "is_private": true }))
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
}

fn add_body(username: &str, role: &str) -> Value {
    json!({ "username": username, "role": role })
}

#[tokio::test]
#[serial]
async fn owner_can_manage_collaborators() {
    let _root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&owner.token);
        let guest = prepare_data::register_and_login(
            &request,
            &ctx,
            "guest@example.com",
            "Guest",
            Some("guest"),
            "secret",
        )
        .await;
        create_private_repo(&request, &key, &value).await;

        let listed = request
            .get("/api/repos/loco/secret/collaborators")
            .add_header(key.clone(), value.clone())
            .await;
        assert_eq!(listed.status_code(), 200, "{}", listed.text());
        let list: Vec<CollaboratorResponse> = serde_json::from_str(&listed.text()).unwrap();
        assert!(list.is_empty());

        let added = request
            .post("/api/repos/loco/secret/collaborators")
            .add_header(key.clone(), value.clone())
            .json(&add_body("guest", "write"))
            .await;
        assert_eq!(added.status_code(), 200, "{}", added.text());
        let added: CollaboratorResponse = serde_json::from_str(&added.text()).unwrap();
        assert_eq!(added.username, "guest");
        assert_eq!(added.user_id, guest.user.id);
        assert_eq!(added.role, "write");

        let updated = request
            .patch(&format!(
                "/api/repos/loco/secret/collaborators/{}",
                added.user_id
            ))
            .add_header(key.clone(), value.clone())
            .json(&json!({ "role": "admin" }))
            .await;
        assert_eq!(updated.status_code(), 200, "{}", updated.text());

        let listed = request
            .get("/api/repos/loco/secret/collaborators")
            .add_header(key.clone(), value.clone())
            .await;
        let list: Vec<CollaboratorResponse> = serde_json::from_str(&listed.text()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].role, "admin");

        let removed = request
            .delete(&format!(
                "/api/repos/loco/secret/collaborators/{}",
                added.user_id
            ))
            .add_header(key.clone(), value.clone())
            .await;
        assert_eq!(removed.status_code(), 200, "{}", removed.text());

        let listed = request
            .get("/api/repos/loco/secret/collaborators")
            .add_header(key, value)
            .await;
        let list: Vec<CollaboratorResponse> = serde_json::from_str(&listed.text()).unwrap();
        assert!(list.is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn invalid_grants_are_rejected() {
    let _root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&owner.token);
        prepare_data::register_and_login(
            &request,
            &ctx,
            "guest@example.com",
            "Guest",
            Some("guest"),
            "secret",
        )
        .await;
        create_private_repo(&request, &key, &value).await;

        for body in [add_body("guest", "superuser"), add_body("loco", "write")] {
            let response = request
                .post("/api/repos/loco/secret/collaborators")
                .add_header(key.clone(), value.clone())
                .json(&body)
                .await;
            assert_eq!(response.status_code(), 400, "{}", response.text());
        }
    })
    .await;
}

#[tokio::test]
#[serial]
async fn non_admins_cannot_manage_collaborators() {
    let _root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (owner_key, owner_value) = prepare_data::auth_header(&owner.token);
        let guest = prepare_data::register_and_login(
            &request,
            &ctx,
            "guest@example.com",
            "Guest",
            Some("guest"),
            "secret",
        )
        .await;
        let (guest_key, guest_value) = prepare_data::auth_header(&guest.token);
        create_private_repo(&request, &owner_key, &owner_value).await;

        let granted = request
            .post("/api/repos/loco/secret/collaborators")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&add_body("guest", "write"))
            .await;
        assert_eq!(granted.status_code(), 200, "{}", granted.text());

        let forbidden = request
            .post("/api/repos/loco/secret/collaborators")
            .add_header(guest_key.clone(), guest_value.clone())
            .json(&add_body("nobody", "read"))
            .await;
        assert_eq!(forbidden.status_code(), 404);

        let forbidden = request
            .delete(&format!(
                "/api/repos/loco/secret/collaborators/{}",
                guest.user.id
            ))
            .add_header(guest_key.clone(), guest_value.clone())
            .await;
        assert_eq!(forbidden.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn collaborator_roles_apply_to_rest_and_git() {
    let _root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (owner_key, owner_value) = prepare_data::auth_header(&owner.token);
        let guest = prepare_data::register_and_login(
            &request,
            &ctx,
            "guest@example.com",
            "Guest",
            Some("guest"),
            "secret",
        )
        .await;
        let (guest_key, guest_value) = prepare_data::auth_header(&guest.token);
        create_private_repo(&request, &owner_key, &owner_value).await;

        let clone = "/loco/secret.git/info/refs?service=git-upload-pack";
        let push = "/loco/secret.git/info/refs?service=git-receive-pack";
        let guest_basic = basic_header("guest", "secret");

        // No grant: a private repo is invisible over REST and git (404, not
        // 401 — the credentials are valid, the account has no access).
        assert_eq!(
            request
                .get("/api/repos/loco/secret")
                .add_header(guest_key.clone(), guest_value.clone())
                .await
                .status_code(),
            404
        );
        assert_eq!(
            request
                .get(clone)
                .add_header(guest_basic.0.clone(), guest_basic.1.clone())
                .await
                .status_code(),
            404
        );

        // Read role: can browse and clone, but not push.
        let granted = request
            .post("/api/repos/loco/secret/collaborators")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&add_body("guest", "read"))
            .await;
        let grant: CollaboratorResponse = serde_json::from_str(&granted.text()).unwrap();

        assert_eq!(
            request
                .get("/api/repos/loco/secret")
                .add_header(guest_key.clone(), guest_value.clone())
                .await
                .status_code(),
            200
        );
        assert_eq!(
            request
                .get(clone)
                .add_header(guest_basic.0.clone(), guest_basic.1.clone())
                .await
                .status_code(),
            200
        );
        assert_eq!(
            request
                .get(push)
                .add_header(guest_basic.0.clone(), guest_basic.1.clone())
                .await
                .status_code(),
            404
        );

        // Write role: push is allowed too.
        request
            .patch(&format!(
                "/api/repos/loco/secret/collaborators/{}",
                grant.user_id
            ))
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({ "role": "write" }))
            .await;
        assert_eq!(
            request
                .get(push)
                .add_header(guest_basic.0.clone(), guest_basic.1.clone())
                .await
                .status_code(),
            200
        );

        // Removal revokes both paths immediately.
        request
            .delete(&format!(
                "/api/repos/loco/secret/collaborators/{}",
                grant.user_id
            ))
            .add_header(owner_key, owner_value)
            .await;
        assert_eq!(
            request
                .get("/api/repos/loco/secret")
                .add_header(guest_key, guest_value)
                .await
                .status_code(),
            404
        );
        assert_eq!(
            request
                .get(clone)
                .add_header(guest_basic.0, guest_basic.1)
                .await
                .status_code(),
            404
        );
    })
    .await;
}
