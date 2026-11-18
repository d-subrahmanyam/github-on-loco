//! Request tests for webhook management and real HTTP delivery.

use axum::{body::Bytes, extract::State, http::HeaderMap, routing::post, Router};
use github_loco::{
    app::App,
    dtos::webhooks::{WebhookDeliveryResponse, WebhookResponse},
};
use loco_rs::testing::prelude::*;
use serde_json::{json, Value};
use serial_test::serial;
use std::sync::{Arc, Mutex};

use super::{prepare_data, support::TestRepoRoot};

#[derive(Debug, Clone)]
struct Received {
    event: String,
    signature: String,
    delivery: String,
    body: String,
}

async fn capture(
    State(store): State<Arc<Mutex<Vec<Received>>>>,
    headers: HeaderMap,
    body: Bytes,
) -> &'static str {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_owned()
    };
    store.lock().unwrap().push(Received {
        event: header("x-github-event"),
        signature: header("x-hub-signature-256"),
        delivery: header("x-github-delivery"),
        body: String::from_utf8_lossy(&body).into_owned(),
    });
    "ok"
}

async fn start_receiver() -> (String, Arc<Mutex<Vec<Received>>>) {
    let store = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route("/hook", post(capture))
        .with_state(store.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{addr}/hook"), store)
}

#[tokio::test]
#[serial]
async fn webhook_settings_are_admin_only_and_secrets_are_masked_after_create() {
    let _root = TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (owner_key, owner_value) = prepare_data::auth_header(&owner.token);
        let created = request
            .post("/api/repositories")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({ "name": "app", "is_private": false }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());

        let writer = prepare_data::register_and_login(
            &request,
            &ctx,
            "writer@example.com",
            "Writer",
            Some("writer"),
            "secret",
        )
        .await;
        let (writer_key, writer_value) = prepare_data::auth_header(&writer.token);
        let granted = request
            .post("/api/repos/loco/app/collaborators")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({ "username": "writer", "role": "write" }))
            .await;
        assert_eq!(granted.status_code(), 200, "{}", granted.text());

        let kinds = request.get("/api/webhook-events").await;
        let kinds: Vec<String> = serde_json::from_str(&kinds.text()).unwrap();
        assert!(kinds.contains(&"star".to_owned()));
        assert!(kinds.contains(&"push".to_owned()));

        let hook = request
            .post("/api/repos/loco/app/webhooks")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({
                "url": "http://127.0.0.1:9999/hook",
                "events": "star,push"
            }))
            .await;
        assert_eq!(hook.status_code(), 200, "{}", hook.text());
        let hook: WebhookResponse = serde_json::from_str(&hook.text()).unwrap();
        assert!(
            hook.secret.starts_with("whsec_"),
            "create reveals the secret"
        );
        assert!(hook.is_active);

        let listed = request
            .get("/api/repos/loco/app/webhooks")
            .add_header(owner_key.clone(), owner_value.clone())
            .await;
        let listed: Vec<WebhookResponse> = serde_json::from_str(&listed.text()).unwrap();
        assert_eq!(listed.len(), 1);
        assert!(
            !listed[0].secret.starts_with("whsec_"),
            "list masks the secret"
        );

        let duplicated = request
            .post("/api/repos/loco/app/webhooks")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({
                "url": "http://127.0.0.1:9999/hook",
                "events": "*"
            }))
            .await;
        assert_eq!(duplicated.status_code(), 400, "{}", duplicated.text());

        let rotated = request
            .post(&format!("/api/repos/loco/app/webhooks/{}/rotate", hook.id))
            .add_header(owner_key.clone(), owner_value.clone())
            .await;
        assert_eq!(rotated.status_code(), 200, "{}", rotated.text());
        let rotated: Value = serde_json::from_str(&rotated.text()).unwrap();
        assert_ne!(rotated["secret"].as_str().unwrap(), hook.secret);

        let updated = request
            .patch(&format!("/api/repos/loco/app/webhooks/{}", hook.id))
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({ "events": "star", "is_active": false }))
            .await;
        assert_eq!(updated.status_code(), 200, "{}", updated.text());
        let updated: WebhookResponse = serde_json::from_str(&updated.text()).unwrap();
        assert_eq!(updated.events, "star");
        assert!(!updated.is_active);

        // A write collaborator and a stranger cannot see or change webhooks.
        let url = "/api/repos/loco/app/webhooks";
        let as_writer = request
            .get(url)
            .add_header(writer_key.clone(), writer_value.clone())
            .await;
        assert_eq!(as_writer.status_code(), 404);
        let as_writer_post = request
            .post(url)
            .add_header(writer_key, writer_value)
            .json(&json!({ "url": "http://x/hook", "events": "*" }))
            .await;
        assert_eq!(as_writer_post.status_code(), 404);

        let deleted = request
            .delete(&format!("/api/repos/loco/app/webhooks/{}", hook.id))
            .add_header(owner_key, owner_value)
            .await;
        assert_eq!(deleted.status_code(), 200, "{}", deleted.text());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn events_are_signed_and_delivered_to_subscribed_webhooks() {
    let _root = TestRepoRoot::new();
    let (url, received) = start_receiver().await;

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&owner.token);
        let created = request
            .post("/api/repositories")
            .add_header(key.clone(), value.clone())
            .json(&json!({ "name": "app", "is_private": false }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());

        let hook = request
            .post("/api/repos/loco/app/webhooks")
            .add_header(key.clone(), value.clone())
            .json(&json!({ "url": url, "events": "star" }))
            .await;
        assert_eq!(hook.status_code(), 200, "{}", hook.text());
        let hook: WebhookResponse = serde_json::from_str(&hook.text()).unwrap();

        // A star emits `star`; the worker posts it inline in ForegroundBlocking mode.
        let starred = request
            .post("/api/repos/loco/app/star")
            .add_header(key.clone(), value.clone())
            .await;
        assert_eq!(starred.status_code(), 200, "{}", starred.text());

        let deliveries = request
            .get(&format!(
                "/api/repos/loco/app/webhooks/{}/deliveries",
                hook.id
            ))
            .add_header(key.clone(), value.clone())
            .await;
        assert_eq!(deliveries.status_code(), 200, "{}", deliveries.text());
        let deliveries: Vec<WebhookDeliveryResponse> =
            serde_json::from_str(&deliveries.text()).unwrap();
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].status, "success");
        assert_eq!(deliveries[0].event, "star");
        assert_eq!(deliveries[0].attempts, 1);

        let seen = received.lock().unwrap().clone();
        assert_eq!(seen.len(), 1, "the webhook should receive exactly one POST");
        assert_eq!(seen[0].event, "star");
        assert!(!seen[0].delivery.is_empty());
        let expected = github_loco::services::webhooks::sign(&hook.secret, &seen[0].body).unwrap();
        assert_eq!(seen[0].signature, expected, "HMAC signature must match");
        let body: Value = serde_json::from_str(&seen[0].body).unwrap();
        assert_eq!(body["event"], "star");
        assert_eq!(body["sender"]["username"], "loco");
        assert_eq!(body["repository"]["name"], "app");
    })
    .await;
}
