//! Release metadata: creation, uniqueness, latest lookup and edits.

use github_loco::{
    app::App,
    models::{
        releases::{self, TAG_NAME_MAX_LEN},
        repositories, users,
        users::RegisterParams,
    },
};
use loco_rs::testing::prelude::*;
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
async fn releases_are_unique_per_tag_and_latest_skips_prereleases() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    let author = make_user(db, "author@example.com", "author").await;
    let repo = repositories::ActiveModel::create_for_user(db, author, "app", None, false)
        .await
        .unwrap();

    let stable = releases::ActiveModel::create(
        db,
        repo.id,
        author,
        "v1.0.0",
        "Version 1.0.0",
        Some("first stable"),
        false,
    )
    .await
    .unwrap();
    assert_eq!(stable.tag_name, "v1.0.0");
    assert_eq!(stable.body.as_deref(), Some("first stable"));
    assert!(stable.published_at.is_some());

    let prerelease = releases::ActiveModel::create(
        db,
        repo.id,
        author,
        "v2.0.0-rc.1",
        "Version 2 RC",
        None,
        true,
    )
    .await
    .unwrap();

    assert_eq!(
        releases::Model::find_by_repository(db, repo.id)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        releases::Model::find_by_tag(db, repo.id, "v1.0.0")
            .await
            .unwrap()
            .unwrap()
            .id,
        stable.id
    );
    // The newer prerelease must not shadow the stable "latest" release.
    assert_eq!(
        releases::Model::find_latest(db, repo.id)
            .await
            .unwrap()
            .unwrap()
            .id,
        stable.id
    );

    assert!(
        releases::ActiveModel::create(db, repo.id, author, "v1.0.0", "Duplicate", None, false,)
            .await
            .is_err(),
        "a tag can only have one release per repository"
    );

    let updated = releases::ActiveModel::update_details(
        db,
        prerelease,
        "Version 2 Release Candidate",
        Some("notes"),
        false,
    )
    .await
    .unwrap();
    assert_eq!(updated.title, "Version 2 Release Candidate");
    assert!(!updated.is_prerelease);
    assert_eq!(
        releases::Model::find_latest(db, repo.id)
            .await
            .unwrap()
            .unwrap()
            .id,
        updated.id
    );

    // Validation happens before the database is touched.
    for tag in [
        "",
        "bad name",
        "../escape",
        &"x".repeat(TAG_NAME_MAX_LEN + 1),
    ] {
        assert!(
            releases::ActiveModel::create(db, repo.id, author, tag, "Title", None, false)
                .await
                .is_err(),
            "invalid tag {tag:?} was accepted"
        );
    }
    assert!(
        releases::ActiveModel::create(db, repo.id, author, "v3", "  ", None, false)
            .await
            .is_err(),
        "blank titles are rejected"
    );
}
