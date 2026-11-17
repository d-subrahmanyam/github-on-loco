//! Request tests for git smart-HTTP visibility rules.

use axum::http::{HeaderName, HeaderValue};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use github_loco::app::App;
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

use super::{prepare_data, support::TestRepoRoot};

#[tokio::test]
#[serial]
async fn private_repos_require_basic_auth_for_clone_and_push() {
    let _root = TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let logged = prepare_data::init_user_login(&request, &ctx).await;
        let (jwt_key, jwt_value) = prepare_data::auth_header(&logged.token);
        let owner = logged.user.owner_handle().to_owned();

        let created = request
            .post("/api/repositories")
            .add_header(jwt_key, jwt_value)
            .json(&json!({ "name": "secret", "is_private": true }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());

        let encoded = STANDARD.encode(format!("{owner}:1234"));
        let auth_value = HeaderValue::from_str(&format!("Basic {encoded}")).unwrap();
        let auth_key = HeaderName::from_static("authorization");

        let clone_url = format!("/{owner}/secret.git/info/refs?service=git-upload-pack");
        let anonymous = request.get(&clone_url).await;
        assert_eq!(
            anonymous.status_code(),
            401,
            "anonymous clone must be rejected"
        );

        let authenticated = request
            .get(&clone_url)
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(authenticated.status_code(), 200, "{}", authenticated.text());

        let push_url = format!("/{owner}/secret.git/info/refs?service=git-receive-pack");
        let push = request.get(&push_url).await;
        assert_eq!(push.status_code(), 401, "anonymous push must be rejected");

        let push_auth = request
            .get(&push_url)
            .add_header(auth_key, auth_value)
            .await;
        assert_eq!(push_auth.status_code(), 200, "{}", push_auth.text());
    })
    .await;
}
