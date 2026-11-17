//! Request tests for personal access token management.

use github_loco::{
    app::App,
    dtos::tokens::{NewTokenResponse, TokenResponse},
};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

use super::prepare_data;

#[tokio::test]
#[serial]
async fn can_issue_list_and_revoke_a_token() {
    request::<App, _, _>(|request, ctx| async move {
        let logged = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&logged.token);

        let created = request
            .post("/api/tokens")
            .add_header(key.clone(), value.clone())
            .json(&json!({
                "name": "laptop",
                "scopes": "repo:read",
                "expires_in_days": 7
            }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());
        let created: NewTokenResponse = serde_json::from_str(&created.text()).unwrap();
        assert!(created.token.starts_with("glp_"));
        assert_eq!(created.info.name, "laptop");

        let listed = request
            .get("/api/tokens")
            .add_header(key.clone(), value.clone())
            .await;
        assert_eq!(listed.status_code(), 200);
        let tokens: Vec<TokenResponse> = serde_json::from_str(&listed.text()).unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].id, created.info.id);
        assert!(
            !listed.text().contains(&created.token),
            "plaintext must never be listed"
        );

        let revoked = request
            .delete(&format!("/api/tokens/{}", created.info.id))
            .add_header(key.clone(), value.clone())
            .await;
        assert_eq!(revoked.status_code(), 200, "{}", revoked.text());

        let listed = request.get("/api/tokens").add_header(key, value).await;
        let tokens: Vec<TokenResponse> = serde_json::from_str(&listed.text()).unwrap();
        assert!(tokens.is_empty());
    })
    .await;
}
