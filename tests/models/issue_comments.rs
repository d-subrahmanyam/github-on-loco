//! Model tests for issue comments.

use github_loco::{
    app::App,
    models::{
        issue_comments, issues, repositories,
        users::{self, RegisterParams},
    },
};
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn creates_and_lists_comments() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let user = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "commenter@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Commenter".to_owned(),
            username: Some("commenter".to_owned()),
        },
    )
    .await
    .unwrap();
    let repo = repositories::ActiveModel::create_for_user(db, user.id, "comments", None, false)
        .await
        .unwrap();
    let (issue, _) =
        issues::ActiveModel::create_for_repository(db, repo.id, user.id, "Discuss", None, &[])
            .await
            .unwrap();

    let comment =
        issue_comments::ActiveModel::create_for_issue(db, issue.id, user.id, "  first comment  ")
            .await
            .unwrap();
    assert_eq!(comment.body, "first comment");

    let found = issue_comments::Entity::find_by_issue(db, issue.id)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(issue.comment_count(db).await.unwrap(), 1);

    assert!(
        issue_comments::ActiveModel::create_for_issue(db, issue.id, user.id, "   ")
            .await
            .is_err()
    );
}
