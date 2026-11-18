//! Model tests for fork edges and source labels.

use github_loco::{
    app::App,
    models::{
        forks::{ActiveModel, Entity},
        repositories,
        users::{self, RegisterParams},
    },
};
use loco_rs::testing::prelude::*;
use sea_orm::EntityTrait;
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
async fn fork_edges_link_source_and_fork() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    Entity::find().all(db).await.unwrap();

    let source_owner = make_user(db, "source@example.com", "sourceowner").await;
    let fork_owner = make_user(db, "fork@example.com", "forkowner").await;
    let source =
        repositories::ActiveModel::create_for_user(db, source_owner, "project", None, false)
            .await
            .unwrap();
    let fork = repositories::ActiveModel::create_for_user(db, fork_owner, "project", None, false)
        .await
        .unwrap();

    let edge = ActiveModel::create(db, source.id, fork.id).await.unwrap();
    assert_eq!(edge.source_repository_id, source.id);
    assert_eq!(edge.repository_id, fork.id);

    assert_eq!(Entity::count_for_source(db, source.id).await.unwrap(), 1);
    assert_eq!(
        Entity::find_by_source(db, source.id).await.unwrap().len(),
        1
    );
    assert!(Entity::find_for_repository(db, fork.id)
        .await
        .unwrap()
        .is_some());
    assert!(Entity::find_for_repository(db, source.id)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        Entity::source_label(db, fork.id).await.unwrap(),
        Some(("sourceowner".to_owned(), "project".to_owned()))
    );
    assert_eq!(Entity::source_label(db, source.id).await.unwrap(), None);
}
