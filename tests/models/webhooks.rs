//! Webhook subscriptions, event matching and delivery bookkeeping.

use github_loco::{
    app::App,
    models::{events, repositories, users, users::RegisterParams, webhook_deliveries, webhooks},
};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

async fn make_user(db: &sea_orm::DatabaseConnection, email: &str, username: &str) -> i64 {
    users::Model::create_with_password(
        db,
        &RegisterParams {
            email: email.to_owned(),
            password: "secret".to_owned(),
            name: username.to_owned(),
            username: Some(username.to_owned()),
        },
    )
    .await
    .unwrap()
    .id
}

#[tokio::test]
#[serial]
async fn webhooks_match_subscriptions_and_record_deliveries() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    let admin = make_user(db, "admin@example.com", "admin").await;
    let repo = repositories::ActiveModel::create_for_user(db, admin, "app", None, false)
        .await
        .unwrap();

    let hook = webhooks::ActiveModel::create(
        db,
        repo.id,
        admin,
        "http://127.0.0.1:9999/hook",
        "star,pr.merged",
    )
    .await
    .unwrap();
    assert_eq!(hook.events, "star,pr.merged");
    assert!(hook.is_active);
    assert!(hook.secret.starts_with("whsec_"));
    assert!(hook.subscribes(events::kind::STAR));
    assert!(!hook.subscribes(events::kind::PUSH));

    let matching = webhooks::Model::find_matching(db, repo.id, events::kind::STAR)
        .await
        .unwrap();
    assert_eq!(matching.len(), 1);
    assert!(
        webhooks::Model::find_matching(db, repo.id, events::kind::PUSH)
            .await
            .unwrap()
            .is_empty()
    );

    // A wildcard subscription matches everything, and duplicates are collapsed.
    let all = webhooks::ActiveModel::create(
        db,
        repo.id,
        admin,
        "http://127.0.0.1:9999/all",
        "*,push,star",
    )
    .await
    .unwrap();
    assert_eq!(all.events, "*");
    assert_eq!(
        webhooks::Model::find_matching(db, repo.id, events::kind::PUSH)
            .await
            .unwrap()
            .len(),
        1
    );

    // Invalid URLs and unknown event kinds are rejected at the model boundary.
    assert!(
        webhooks::ActiveModel::create(db, repo.id, admin, "ftp://x", "*",)
            .await
            .is_err()
    );
    assert!(
        webhooks::ActiveModel::create(db, repo.id, admin, "http://x/other", "nope")
            .await
            .is_err()
    );
    assert!(
        webhooks::ActiveModel::create(db, repo.id, admin, "http://127.0.0.1:9999/hook", "*",)
            .await
            .is_err(),
        "a duplicate URL is rejected"
    );

    let updated = webhooks::ActiveModel::update_subscription(db, hook, "star", false)
        .await
        .unwrap();
    assert!(!updated.is_active);
    assert!(
        webhooks::Model::find_matching(db, repo.id, events::kind::STAR)
            .await
            .unwrap()
            .len()
            == 1,
        "the inactive hook no longer matches"
    );
    let rotated = webhooks::ActiveModel::rotate_secret(db, updated)
        .await
        .unwrap();
    assert_ne!(rotated.secret, all.secret);

    let delivery = webhook_deliveries::ActiveModel::create(
        db,
        rotated.id,
        events::kind::STAR,
        &json!({"x": 1}),
    )
    .await
    .unwrap();
    assert_eq!(delivery.status, webhook_deliveries::STATUS_PENDING);
    let failed = webhook_deliveries::ActiveModel::mark_failed(db, delivery, "HTTP 500")
        .await
        .unwrap();
    assert_eq!(failed.status, webhook_deliveries::STATUS_FAILED);
    assert_eq!(failed.attempts, 1);
    let success = webhook_deliveries::ActiveModel::mark_success(db, failed)
        .await
        .unwrap();
    assert_eq!(success.status, webhook_deliveries::STATUS_SUCCESS);
    assert_eq!(success.attempts, 2);
    assert!(success.delivered_at.is_some());
    assert_eq!(
        webhook_deliveries::Model::find_by_webhook(db, rotated.id, 10)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn signatures_are_deterministic_and_keyed() {
    let a = github_loco::services::webhooks::sign("secret", "{\"event\":\"push\"}").unwrap();
    let b = github_loco::services::webhooks::sign("secret", "{\"event\":\"push\"}").unwrap();
    let other = github_loco::services::webhooks::sign("other", "{\"event\":\"push\"}").unwrap();
    assert_eq!(a, b);
    assert_ne!(a, other);
    assert!(a.starts_with("sha256="));
}
