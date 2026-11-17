use github_loco::{app::App, models::_entities::branches};
use loco_rs::testing::prelude::*;
use sea_orm::EntityTrait;
use serial_test::serial;

macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        let _guard = settings.bind_to_scope();
    };
}

/// The `Branches` entity matches the table its migration created.
///
/// Selecting every column is the cheapest assertion that says something true:
/// it fails if the migration and the entity disagree — a renamed column, a
/// type that does not round-trip, a migration that never ran — which is the
/// most common way a generated model breaks. Extend it as the model grows.
#[tokio::test]
#[serial]
async fn can_query_branches() {
    configure_insta!();

    let boot = boot_test::<App>().await.unwrap();
    seed::<App>(&boot.app_context).await.unwrap();

    // The query is the assertion. Bind the result and compare it once you have
    // seed data to compare against, e.g.:
    //
    // let items = branches::Entity::find().all(&boot.app_context.db).await.unwrap();
    // assert_debug_snapshot!(items);
    branches::Entity::find()
        .all(&boot.app_context.db)
        .await
        .expect("`branches` should be queryable — entity and migration must agree");
}

#[tokio::test]
#[serial]
async fn can_replace_and_clear_the_branch_cache() {
    use github_loco::dtos::repositories::Branch;
    use github_loco::models::{
        branches as branch_model, repositories,
        users::{self, RegisterParams},
    };

    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let user = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "branch-cache@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Branch Cache".to_owned(),
            username: Some("branch-cache".to_owned()),
        },
    )
    .await
    .unwrap();
    let repo = repositories::ActiveModel::create_for_user(db, user.id, "cached", None, false)
        .await
        .unwrap();

    branch_model::Entity::replace_for_repository(
        db,
        repo.id,
        &[Branch {
            name: "main".to_owned(),
            sha: "abc123".to_owned(),
        }],
    )
    .await
    .unwrap();

    let cached = branch_model::Entity::find_by_repository(db, repo.id)
        .await
        .unwrap();
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0].name, "main");

    branch_model::Entity::clear_for_repository(db, repo.id)
        .await
        .unwrap();
    assert!(branch_model::Entity::find_by_repository(db, repo.id)
        .await
        .unwrap()
        .is_empty());
}
