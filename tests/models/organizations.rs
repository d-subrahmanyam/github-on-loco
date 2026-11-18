//! Organization membership, teams, grants and the resulting repository access.

use github_loco::{
    app::App,
    models::{
        org_members, organizations, repositories,
        repositories::RepoAction,
        team_members, team_repositories, teams,
        users::{self, RegisterParams},
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
async fn org_roles_and_team_grants_drive_repository_access() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    let owner_id = make_user(db, "owner@example.com", "orgowner").await;
    let member_id = make_user(db, "member@example.com", "orgmember").await;
    let outsider_id = make_user(db, "outsider@example.com", "outsider").await;

    let org = organizations::ActiveModel::create_with_owner(
        db,
        owner_id,
        "acme",
        "Acme Inc.",
        Some("widgets"),
    )
    .await
    .unwrap();
    assert!(org.is_owner(db, owner_id).await.unwrap());
    assert_eq!(org.role_for(db, member_id).await.unwrap(), None);
    assert_eq!(
        organizations::Entity::find_by_handle(db, "acme")
            .await
            .unwrap()
            .id,
        org.id
    );
    assert_eq!(
        organizations::Entity::find_by_user(db, member_id)
            .await
            .unwrap()
            .len(),
        0
    );

    org_members::ActiveModel::add(db, org.id, member_id, organizations::ROLE_MEMBER)
        .await
        .unwrap();
    assert_eq!(
        organizations::Entity::find_by_user(db, member_id)
            .await
            .unwrap()
            .len(),
        1
    );

    let team = teams::ActiveModel::create(db, org.id, "backend", None)
        .await
        .unwrap();
    team_members::ActiveModel::add(db, &team, member_id)
        .await
        .unwrap();
    assert!(team_members::Entity::contains(db, team.id, member_id)
        .await
        .unwrap());

    let repo =
        repositories::ActiveModel::create_for_organization(db, org.id, owner_id, "api", None, true)
            .await
            .unwrap();
    assert!(repo.is_org_owned());
    assert!(!repo.is_owned_by(owner_id)); // creator is not automatically owner
    assert_eq!(
        repositories::Model::find_by_owner_and_name(db, "acme", "api")
            .await
            .unwrap()
            .id,
        repo.id
    );

    // Owner has all actions; a plain member has none until a team gets a grant.
    assert!(repo
        .can(db, Some(owner_id), RepoAction::Admin)
        .await
        .unwrap());
    assert!(!repo
        .can(db, Some(member_id), RepoAction::Read)
        .await
        .unwrap());
    assert!(!repo
        .can(db, Some(outsider_id), RepoAction::Read)
        .await
        .unwrap());
    assert_eq!(repo.role_for(db, Some(owner_id)).await.unwrap(), "owner");

    team_repositories::ActiveModel::grant(db, &team, repo.id, repositories::ROLE_WRITE)
        .await
        .unwrap();
    assert!(repo
        .can(db, Some(member_id), RepoAction::Write)
        .await
        .unwrap());
    assert!(!repo
        .can(db, Some(member_id), RepoAction::Admin)
        .await
        .unwrap());
    assert_eq!(repo.role_for(db, Some(member_id)).await.unwrap(), "write");

    // Removing the team membership revokes the derived access immediately.
    team_members::ActiveModel::remove(db, team.id, member_id)
        .await
        .unwrap();
    assert!(!repo
        .can(db, Some(member_id), RepoAction::Write)
        .await
        .unwrap());
    assert_eq!(repo.role_for(db, Some(member_id)).await.unwrap(), "none");

    // The last owner cannot be removed, even by themselves.
    assert!(org_members::ActiveModel::remove(db, org.id, owner_id)
        .await
        .is_err());
    assert_eq!(
        org_members::Entity::owner_count(db, org.id).await.unwrap(),
        1
    );
}
