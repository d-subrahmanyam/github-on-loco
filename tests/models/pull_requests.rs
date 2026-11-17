//! Model tests for pull requests: numbering, state and merge recording.

use github_loco::{
    app::App,
    models::{
        pull_requests, repositories,
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
            email: "pr-author@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "PR Author".to_owned(),
            username: Some("pr-author".to_owned()),
        },
    )
    .await
    .unwrap();
    let repo = repositories::ActiveModel::create_for_user(db, user.id, "prs", None, false)
        .await
        .unwrap();
    (boot.app_context, user.id, repo.id)
}

#[tokio::test]
#[serial]
async fn pull_requests_are_numbered_per_repository_and_track_state() {
    let (ctx, user_id, repo_id) = setup().await;
    let db = &ctx.db;

    let first = pull_requests::ActiveModel::create_for_repository(
        db,
        repo_id,
        user_id,
        "First PR",
        Some("body"),
        "feature",
        "abc123",
        "main",
    )
    .await
    .unwrap();
    let second = pull_requests::ActiveModel::create_for_repository(
        db,
        repo_id,
        user_id,
        "Second PR",
        None,
        "other",
        "def456",
        "main",
    )
    .await
    .unwrap();

    assert_eq!(first.number, 1);
    assert_eq!(second.number, 2);
    assert!(first.is_open());
    assert!(!first.is_merged());
    assert_eq!(first.head_sha, "abc123");

    let closed = pull_requests::ActiveModel {
        id: Set(first.id),
        ..Default::default()
    }
    .set_state(db, pull_requests::STATE_CLOSED)
    .await
    .unwrap();
    assert_eq!(closed.state, pull_requests::STATE_CLOSED);

    let merged = pull_requests::ActiveModel {
        id: Set(second.id),
        ..Default::default()
    }
    .mark_merged(db, "newsha")
    .await
    .unwrap();
    assert!(merged.is_merged());
    assert_eq!(merged.merged_commit_sha.as_deref(), Some("newsha"));
    assert!(merged.merged_at.is_some());

    let open =
        pull_requests::Model::find_by_repository(db, repo_id, Some(pull_requests::STATE_OPEN))
            .await
            .unwrap();
    assert!(open.is_empty());
    let merged_list =
        pull_requests::Model::find_by_repository(db, repo_id, Some(pull_requests::STATE_MERGED))
            .await
            .unwrap();
    assert_eq!(merged_list.len(), 1);
}

#[tokio::test]
#[serial]
async fn rejects_empty_titles_and_identical_branches() {
    let (ctx, user_id, repo_id) = setup().await;
    let db = &ctx.db;

    let empty = pull_requests::ActiveModel::create_for_repository(
        db, repo_id, user_id, "  ", None, "feature", "abc", "main",
    )
    .await;
    assert!(empty.is_err());

    let same = pull_requests::ActiveModel::create_for_repository(
        db, repo_id, user_id, "Title", None, "main", "abc", "main",
    )
    .await;
    assert!(same.is_err());
}
