//! Model tests for repository labels.

use github_loco::{
    app::App,
    models::{
        labels, repositories,
        users::{self, RegisterParams},
    },
};
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn creates_and_lists_labels_for_a_repository() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let user = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "label-owner@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Label Owner".to_owned(),
            username: Some("label-owner".to_owned()),
        },
    )
    .await
    .unwrap();
    let repo = repositories::ActiveModel::create_for_user(db, user.id, "labels", None, false)
        .await
        .unwrap();

    let label = labels::ActiveModel::create_for_repository(db, repo.id, "bug", "#ff0000")
        .await
        .unwrap();
    assert_eq!(label.name, "bug");
    assert_eq!(label.color, "ff0000");

    let found = labels::Entity::find_by_repository(db, repo.id)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(
        labels::Entity::find_by_name(db, repo.id, "bug")
            .await
            .unwrap()
            .unwrap()
            .id,
        label.id
    );
}

#[tokio::test]
#[serial]
async fn rejects_invalid_label_colors() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let user = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "bad-color@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Bad Color".to_owned(),
            username: Some("bad-color".to_owned()),
        },
    )
    .await
    .unwrap();
    let repo = repositories::ActiveModel::create_for_user(db, user.id, "colors", None, false)
        .await
        .unwrap();

    for color in ["red", "fff", "gggggg"] {
        assert!(
            labels::ActiveModel::create_for_repository(db, repo.id, "x", color)
                .await
                .is_err()
        );
    }
}
