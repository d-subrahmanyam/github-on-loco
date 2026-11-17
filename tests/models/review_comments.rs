//! Model tests for review comments and their PR-scoped finder.

use github_loco::{
    app::App,
    models::{
        pull_requests, repositories, review_comments,
        users::{self, RegisterParams},
    },
};
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn review_comments_are_scoped_to_a_pull_request() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let user = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "reviewer@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Reviewer".to_owned(),
            username: Some("reviewer".to_owned()),
        },
    )
    .await
    .unwrap();
    let repo = repositories::ActiveModel::create_for_user(db, user.id, "reviews", None, false)
        .await
        .unwrap();
    let pr = pull_requests::ActiveModel::create_for_repository(
        db,
        repo.id,
        user.id,
        "Review me",
        None,
        "feature",
        "abc",
        "main",
    )
    .await
    .unwrap();

    let general =
        review_comments::ActiveModel::create_for_pr(db, pr.id, user.id, "LGTM", None, None)
            .await
            .unwrap();
    let anchored = review_comments::ActiveModel::create_for_pr(
        db,
        pr.id,
        user.id,
        "Nit",
        Some("main.rs"),
        Some(12),
    )
    .await
    .unwrap();

    assert_eq!(general.path, None);
    assert_eq!(anchored.path.as_deref(), Some("main.rs"));
    assert_eq!(anchored.line, Some(12));

    let comments = review_comments::Entity::find_by_pr(db, pr.id)
        .await
        .unwrap();
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0].id, general.id);
    assert_eq!(comments[1].id, anchored.id);

    assert!(
        review_comments::ActiveModel::create_for_pr(db, pr.id, user.id, "  ", None, None)
            .await
            .is_err()
    );
}
