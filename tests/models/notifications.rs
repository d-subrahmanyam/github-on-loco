//! Notification rows and status transitions.

use github_loco::{
    app::App,
    models::{notifications, repositories, users, users::RegisterParams},
};
use loco_rs::testing::prelude::*;
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
async fn notifications_are_per_recipient_channel_and_track_attempts() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    let owner = make_user(db, "owner@example.com", "owner").await;
    let watcher = make_user(db, "watcher@example.com", "watcher").await;
    let repo = repositories::ActiveModel::create_for_user(db, owner, "app", None, false)
        .await
        .unwrap();

    let row = notifications::ActiveModel::create(
        db,
        watcher,
        repo.id,
        "push",
        "New commits in owner/app",
        "owner pushed to main",
        "log",
    )
    .await
    .unwrap();
    assert_eq!(row.status, notifications::STATUS_PENDING);
    assert_eq!(row.attempts, 0);
    assert_eq!(row.channel, "log");

    let sent = notifications::ActiveModel::mark_sent(db, row)
        .await
        .unwrap();
    assert_eq!(sent.status, notifications::STATUS_SENT);
    assert_eq!(sent.attempts, 1);
    assert!(sent.delivered_at.is_some());

    let failed = notifications::ActiveModel::mark_failed(db, sent, "HTTP 500")
        .await
        .unwrap();
    assert_eq!(failed.status, notifications::STATUS_FAILED);
    assert_eq!(failed.attempts, 2);
    assert_eq!(failed.last_error.as_deref(), Some("HTTP 500"));

    let history = notifications::Model::find_by_user(db, watcher, 10)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert!(notifications::Model::find_by_user(db, owner, 10)
        .await
        .unwrap()
        .is_empty());
}
