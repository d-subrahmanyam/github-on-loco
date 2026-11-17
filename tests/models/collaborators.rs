//! Model tests for collaborator grants and the unified `can` rule.

use github_loco::{
    app::App,
    models::{
        collaborators, repositories,
        repositories::RepoAction,
        users::{self, RegisterParams},
    },
};
use loco_rs::testing::prelude::*;
use sea_orm::{ActiveValue::Set, EntityTrait};
use serial_test::serial;

async fn setup() -> (loco_rs::app::AppContext, i64, i64, i64) {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let owner = users::Model::create_with_password(
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
    let collaborator = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "collab@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Collaborator".to_owned(),
            username: Some("collab".to_owned()),
        },
    )
    .await
    .unwrap();
    let repo = repositories::ActiveModel::create_for_user(db, owner.id, "private", None, true)
        .await
        .unwrap();
    (boot.app_context, owner.id, collaborator.id, repo.id)
}

#[tokio::test]
#[serial]
async fn grants_drive_the_single_can_rule() {
    let (ctx, owner_id, user_id, repo_id) = setup().await;
    let db = &ctx.db;
    let repo = repositories::Entity::find_by_id(repo_id)
        .one(db)
        .await
        .unwrap()
        .unwrap();

    // Owner has every action; an ungranted user has none, even read.
    assert!(repo
        .can(db, Some(owner_id), RepoAction::Admin)
        .await
        .unwrap());
    assert!(!repo.can_read(db, Some(user_id)).await.unwrap());

    let grant = collaborators::ActiveModel::add(db, repo_id, user_id, repositories::ROLE_READ)
        .await
        .unwrap();
    assert!(grant.allows(RepoAction::Read));
    assert!(!grant.allows(RepoAction::Write));
    assert!(repo.can_read(db, Some(user_id)).await.unwrap());
    assert!(!repo.can_write(db, user_id).await.unwrap());

    let active = collaborators::ActiveModel {
        id: Set(grant.id),
        ..Default::default()
    };
    let write = active.set_role(db, repositories::ROLE_WRITE).await.unwrap();
    assert!(write.allows(RepoAction::Write));
    assert!(repo.can_write(db, user_id).await.unwrap());
    assert!(!repo.can_admin(db, user_id).await.unwrap());

    let active = collaborators::ActiveModel {
        id: Set(grant.id),
        ..Default::default()
    };
    active.set_role(db, repositories::ROLE_ADMIN).await.unwrap();
    assert!(repo.can_admin(db, user_id).await.unwrap());
    assert_eq!(repo.role_for(db, Some(user_id)).await.unwrap(), "admin");

    collaborators::Entity::remove(db, repo_id, user_id)
        .await
        .unwrap();
    assert!(!repo.can_read(db, Some(user_id)).await.unwrap());
    assert_eq!(repo.role_for(db, Some(user_id)).await.unwrap(), "none");
}

#[tokio::test]
#[serial]
async fn invalid_roles_and_duplicate_grants_are_rejected() {
    let (ctx, _owner_id, user_id, repo_id) = setup().await;
    let db = &ctx.db;

    assert!(
        collaborators::ActiveModel::add(db, repo_id, user_id, "superuser")
            .await
            .is_err()
    );
    collaborators::ActiveModel::add(db, repo_id, user_id, repositories::ROLE_READ)
        .await
        .unwrap();
    assert!(
        collaborators::ActiveModel::add(db, repo_id, user_id, repositories::ROLE_WRITE)
            .await
            .is_err()
    );
    assert_eq!(
        collaborators::Entity::find_by_repository(db, repo_id)
            .await
            .unwrap()
            .len(),
        1
    );
}
