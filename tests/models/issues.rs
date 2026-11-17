//! Model tests for issues: numbering, labels and state transitions.

use github_loco::{
    app::App,
    models::{
        issue_labels, issues, labels, repositories,
        users::{self, RegisterParams},
    },
};
use loco_rs::testing::prelude::*;
use sea_orm::ActiveValue::Set;
use serial_test::serial;

async fn setup() -> (loco_rs::app::AppContext, i64, i64) {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let user = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "issue-author@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Issue Author".to_owned(),
            username: Some("issue-author".to_owned()),
        },
    )
    .await
    .unwrap();
    let repo = repositories::ActiveModel::create_for_user(db, user.id, "issues", None, false)
        .await
        .unwrap();
    (boot.app_context, user.id, repo.id)
}

#[tokio::test]
#[serial]
async fn issues_are_numbered_per_repository_and_can_change_state() {
    let (ctx, user_id, repo_id) = setup().await;
    let db = &ctx.db;

    let (first, _) = issues::ActiveModel::create_for_repository(
        db,
        repo_id,
        user_id,
        "First issue",
        Some("body"),
        &[],
    )
    .await
    .unwrap();
    let (second, _) =
        issues::ActiveModel::create_for_repository(db, repo_id, user_id, "Second issue", None, &[])
            .await
            .unwrap();

    assert_eq!(first.number, 1);
    assert_eq!(second.number, 2);
    assert!(first.is_open());

    let closed = issues::ActiveModel {
        id: Set(first.id),
        ..Default::default()
    }
    .set_state(db, issues::STATE_CLOSED)
    .await
    .unwrap();
    assert!(!closed.is_open());

    let list = issues::Model::find_by_repository(db, repo_id, Some(issues::STATE_OPEN))
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].number, 2);
}

#[tokio::test]
#[serial]
async fn creating_an_issue_attaches_its_labels() {
    let (ctx, user_id, repo_id) = setup().await;
    let db = &ctx.db;
    let label = labels::ActiveModel::create_for_repository(db, repo_id, "bug", "ff0000")
        .await
        .unwrap();

    let (issue, attached) = issues::ActiveModel::create_for_repository(
        db,
        repo_id,
        user_id,
        "Broken",
        None,
        &[label.id],
    )
    .await
    .unwrap();

    assert_eq!(attached.len(), 1);
    let found = issue_labels::Entity::labels_for_issue(db, issue.id)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "bug");
}

#[tokio::test]
#[serial]
async fn rejects_a_label_from_another_repository() {
    let (ctx, user_id, repo_id) = setup().await;
    let db = &ctx.db;
    let foreign = repositories::ActiveModel::create_for_user(db, user_id, "other", None, false)
        .await
        .unwrap();
    let label = labels::ActiveModel::create_for_repository(db, foreign.id, "foreign", "00ff00")
        .await
        .unwrap();

    assert!(issues::ActiveModel::create_for_repository(
        db,
        repo_id,
        user_id,
        "Bad",
        None,
        &[label.id],
    )
    .await
    .is_err());
}
