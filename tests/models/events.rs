//! Model tests for the append-only activity log.

use github_loco::{
    app::App,
    models::{
        events::{self, Entity},
        repositories,
        users::{self, RegisterParams},
    },
};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn events_are_recorded_and_returned_newest_first() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let user = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "actor@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Actor".to_owned(),
            username: Some("actor".to_owned()),
        },
    )
    .await
    .unwrap();
    let repo = repositories::ActiveModel::create_for_user(db, user.id, "events", None, false)
        .await
        .unwrap();

    let first = Entity::record(
        db,
        repo.id,
        user.id,
        events::kind::PUSH,
        Some(json!({ "branch": "main" })),
    )
    .await
    .unwrap();
    let second = Entity::record(db, repo.id, user.id, events::kind::STAR, None)
        .await
        .unwrap();

    assert_eq!(first.payload, Some(json!({ "branch": "main" })));
    assert_eq!(second.payload, None);

    let all = Entity::find_by_repository(db, repo.id, 20).await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].id, second.id); // newest first
    assert_eq!(all[1].id, first.id);

    let limited = Entity::find_by_repository(db, repo.id, 1).await.unwrap();
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].id, second.id);
}
