//! Model tests for the issue/label join table.

use github_loco::{
    app::App,
    models::{
        issue_labels, issues, labels, repositories,
        users::{self, RegisterParams},
    },
};
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn replaces_an_issues_labels() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let user = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "join@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Join".to_owned(),
            username: Some("join".to_owned()),
        },
    )
    .await
    .unwrap();
    let repo = repositories::ActiveModel::create_for_user(db, user.id, "join", None, false)
        .await
        .unwrap();
    let bug = labels::ActiveModel::create_for_repository(db, repo.id, "bug", "ff0000")
        .await
        .unwrap();
    let docs = labels::ActiveModel::create_for_repository(db, repo.id, "docs", "0000ff")
        .await
        .unwrap();
    let (issue, _) =
        issues::ActiveModel::create_for_repository(db, repo.id, user.id, "Topic", None, &[])
            .await
            .unwrap();

    issue_labels::Entity::replace_for_issue(db, issue.id, &[bug.id, docs.id])
        .await
        .unwrap();
    let found = issue_labels::Entity::labels_for_issue(db, issue.id)
        .await
        .unwrap();
    assert_eq!(found.len(), 2);

    issue_labels::Entity::replace_for_issue(db, issue.id, &[docs.id])
        .await
        .unwrap();
    let found = issue_labels::Entity::labels_for_issue(db, issue.id)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "docs");
}
