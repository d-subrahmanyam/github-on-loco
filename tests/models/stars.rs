//! Model tests for stars: idempotence and counts.

use github_loco::{
    app::App,
    models::{
        repositories,
        stars::{ActiveModel, Entity},
        users::{self, RegisterParams},
    },
};
use loco_rs::testing::prelude::*;
use serial_test::serial;

async fn setup() -> (loco_rs::app::AppContext, i64, i64) {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let user = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "starrer@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Starrer".to_owned(),
            username: Some("starrer".to_owned()),
        },
    )
    .await
    .unwrap();
    let repo = repositories::ActiveModel::create_for_user(db, user.id, "starred", None, false)
        .await
        .unwrap();
    (boot.app_context, user.id, repo.id)
}

#[tokio::test]
#[serial]
async fn starring_is_idempotent_and_counted() {
    let (ctx, user_id, repo_id) = setup().await;
    let db = &ctx.db;

    assert!(!Entity::is_starred(db, repo_id, user_id).await.unwrap());
    ActiveModel::star(db, repo_id, user_id).await.unwrap();
    assert!(Entity::is_starred(db, repo_id, user_id).await.unwrap());
    assert_eq!(Entity::count_for_repository(db, repo_id).await.unwrap(), 1);

    // A repeated star does not create a second row.
    ActiveModel::star(db, repo_id, user_id).await.unwrap();
    assert_eq!(Entity::count_for_repository(db, repo_id).await.unwrap(), 1);

    Entity::unstar(db, repo_id, user_id).await.unwrap();
    assert!(!Entity::is_starred(db, repo_id, user_id).await.unwrap());
    assert_eq!(Entity::count_for_repository(db, repo_id).await.unwrap(), 0);

    // Unstarring twice is harmless.
    Entity::unstar(db, repo_id, user_id).await.unwrap();
}
