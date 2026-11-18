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

#[tokio::test]
#[serial]
async fn git_auth_errors_distinguish_missing_repos_from_missing_permissions() {
    let _root = TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (jwt_key, jwt_value) = prepare_data::auth_header(&owner.token);
        let owner_handle = owner.user.owner_handle().to_owned();

        let stranger = prepare_data::register_and_login(
            &request,
            &ctx,
            "stranger@example.com",
            "Stranger",
            Some("stranger"),
            "1234",
        )
        .await;
        assert_ne!(stranger.user.id, owner.user.id);

        for (name, is_private) in [("public-repo", false), ("private-repo", true)] {
            let created = request
                .post("/api/repositories")
                .add_header(jwt_key.clone(), jwt_value.clone())
                .json(&json!({ "name": name, "is_private": is_private }))
                .await;
            assert_eq!(created.status_code(), 200, "{}", created.text());
        }

        let auth_key = HeaderName::from_static("authorization");
        let basic = |login: &str, password: &str| {
            let encoded = STANDARD.encode(format!("{login}:{password}"));
            HeaderValue::from_str(&format!("Basic {encoded}")).unwrap()
        };
        let push_url = |owner: &str, repo: &str| {
            format!("/{owner}/{repo}.git/info/refs?service=git-receive-pack")
        };
        let stranger_basic = basic("stranger", "1234");

        // A missing repo is opaque to anonymous callers: same 401 as a private
        // one, so probing cannot learn whether a path exists.
        let missing_anonymous = request.get(&push_url("nobody", "missing")).await;
        assert_eq!(missing_anonymous.status_code(), 401);

        // An authenticated caller gets the real reason instead.
        let missing_auth = request
            .get(&push_url("nobody", "missing"))
            .add_header(auth_key.clone(), stranger_basic.clone())
            .await;
        assert_eq!(missing_auth.status_code(), 404, "{}", missing_auth.text());
        assert!(
            missing_auth.text().contains("repository not found"),
            "{}",
            missing_auth.text()
        );

        // Valid credentials without write access to a public repo: 403 with a
        // reason, and no WWW-Authenticate so git does not re-prompt.
        let denied = request
            .get(&push_url(&owner_handle, "public-repo"))
            .add_header(auth_key.clone(), stranger_basic.clone())
            .await;
        assert_eq!(denied.status_code(), 403, "{}", denied.text());
        assert!(denied.text().contains("write access"), "{}", denied.text());
        assert!(denied.headers().get("www-authenticate").is_none());

        // A private repo still hides its existence from an unauthorized user.
        let private_anonymous = request.get(&push_url(&owner_handle, "private-repo")).await;
        assert_eq!(private_anonymous.status_code(), 401);
        let private_denied = request
            .get(&push_url(&owner_handle, "private-repo"))
            .add_header(auth_key.clone(), stranger_basic)
            .await;
        assert_eq!(
            private_denied.status_code(),
            404,
            "{}",
            private_denied.text()
        );

        // The owner keeps working push access to both repositories.
        let owner_basic = basic(&owner_handle, "1234");
        for repo in ["public-repo", "private-repo"] {
            let allowed = request
                .get(&push_url(&owner_handle, repo))
                .add_header(auth_key.clone(), owner_basic.clone())
                .await;
            assert_eq!(allowed.status_code(), 200, "{}", allowed.text());
        }
    })
    .await;
}
