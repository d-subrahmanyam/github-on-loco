//! Model tests for branch protection rules: upsert, validation and removal.

use github_loco::{
    app::App,
    models::{
        branch_protection_rules::{ActiveModel, Entity, MAX_REQUIRED_APPROVALS},
        repositories,
        users::{self, RegisterParams},
    },
};
use loco_rs::testing::prelude::*;
use sea_orm::EntityTrait;
use serial_test::serial;

async fn setup() -> (loco_rs::app::AppContext, i64) {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let user = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "owner@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Owner".to_owned(),
            username: Some("owner".to_owned()),
        },
    )
    .await
    .unwrap();
    let repo = repositories::ActiveModel::create_for_user(db, user.id, "protected", None, false)
        .await
        .unwrap();
    (boot.app_context, repo.id)
}

#[tokio::test]
#[serial]
async fn rules_can_be_upserted_validated_and_removed() {
    let (ctx, repo_id) = setup().await;
    let db = &ctx.db;

    // The generated query is the cheapest assertion that entity and migration
    // still agree on the schema.
    Entity::find().all(db).await.unwrap();

    let rule = ActiveModel::upsert(db, repo_id, "main", true, 2)
        .await
        .unwrap();
    assert!(rule.require_pull_request());
    assert_eq!(rule.required_approvals(), 2);
    assert_eq!(
        Entity::find_by_repository(db, repo_id).await.unwrap().len(),
        1
    );

    // Changing the settings updates the same row rather than inserting another.
    let updated = ActiveModel::upsert(db, repo_id, "main", false, 3)
        .await
        .unwrap();
    assert_eq!(updated.id, rule.id);
    assert!(!updated.require_pull_request());
    assert_eq!(updated.required_approvals(), 3);
    assert_eq!(
        Entity::find_by_repository(db, repo_id).await.unwrap().len(),
        1
    );

    // A second branch gets its own rule.
    ActiveModel::upsert(db, repo_id, "release/1.x", true, 1)
        .await
        .unwrap();
    assert_eq!(
        Entity::find_by_repository(db, repo_id).await.unwrap().len(),
        2
    );
    assert!(Entity::find_for_branch(db, repo_id, "release/1.x")
        .await
        .unwrap()
        .is_some());

    assert!(ActiveModel::upsert(db, repo_id, "a/../b", true, 1)
        .await
        .is_err());
    assert!(
        ActiveModel::upsert(db, repo_id, "main", true, MAX_REQUIRED_APPROVALS + 1)
            .await
            .is_err()
    );

    ActiveModel::remove(db, repo_id, "main").await.unwrap();
    assert!(Entity::find_for_branch(db, repo_id, "main")
        .await
        .unwrap()
        .is_none());
    // Removing a missing rule is harmless.
    ActiveModel::remove(db, repo_id, "main").await.unwrap();
}
