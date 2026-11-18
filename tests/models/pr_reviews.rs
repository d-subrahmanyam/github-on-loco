//! Model tests for pull request reviews: one verdict per reviewer, and the
//! approval count used by branch protection.

use github_loco::{
    app::App,
    models::{
        pr_reviews::{state, ActiveModel, Entity},
        pull_requests, repositories,
        users::{self, RegisterParams},
    },
};
use loco_rs::testing::prelude::*;
use sea_orm::EntityTrait;
use serial_test::serial;

async fn make_user(
    db: &sea_orm::DatabaseConnection,
    email: &str,
    name: &str,
    username: &str,
) -> i64 {
    users::Model::create_with_password(
        db,
        &RegisterParams {
            email: email.to_owned(),
            password: "secret".to_owned(),
            name: name.to_owned(),
            username: Some(username.to_owned()),
        },
    )
    .await
    .unwrap()
    .id
}

#[tokio::test]
#[serial]
async fn reviews_are_upserted_and_only_non_author_approvals_count() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    Entity::find().all(db).await.unwrap();

    let author_id = make_user(db, "author@example.com", "Author", "author").await;
    let reviewer_id = make_user(db, "reviewer@example.com", "Reviewer", "reviewer").await;
    let repo = repositories::ActiveModel::create_for_user(db, author_id, "reviews", None, false)
        .await
        .unwrap();
    let pr = pull_requests::ActiveModel::create_for_repository(
        db, repo.id, author_id, "A change", None, "feature", "abc123", "main",
    )
    .await
    .unwrap();

    assert_eq!(
        Entity::approval_count(db, pr.id, author_id).await.unwrap(),
        0
    );

    let approved = ActiveModel::set_review(db, pr.id, reviewer_id, state::APPROVED, None)
        .await
        .unwrap();
    assert!(approved.is_approval());
    assert_eq!(
        Entity::approval_count(db, pr.id, author_id).await.unwrap(),
        1
    );

    // Changing the verdict replaces the row and drops the approval.
    let changed = ActiveModel::set_review(
        db,
        pr.id,
        reviewer_id,
        state::CHANGES_REQUESTED,
        Some("nope"),
    )
    .await
    .unwrap();
    assert_eq!(changed.id, approved.id);
    assert_eq!(changed.body.as_deref(), Some("nope"));
    assert_eq!(
        Entity::approval_count(db, pr.id, author_id).await.unwrap(),
        0
    );
    assert_eq!(Entity::find_by_pr(db, pr.id).await.unwrap().len(), 1);

    ActiveModel::set_review(db, pr.id, reviewer_id, state::APPROVED, None)
        .await
        .unwrap();

    // A self-approval is stored but never counted, so legacy rows cannot
    // satisfy a protection rule on their own.
    ActiveModel::set_review(db, pr.id, author_id, state::APPROVED, None)
        .await
        .unwrap();
    assert_eq!(
        Entity::approval_count(db, pr.id, author_id).await.unwrap(),
        1
    );
    assert_eq!(Entity::find_by_pr(db, pr.id).await.unwrap().len(), 2);

    assert!(
        ActiveModel::set_review(db, pr.id, reviewer_id, "lgtm", None)
            .await
            .is_err()
    );
}
