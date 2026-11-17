//! Model tests for personal access tokens.

use chrono::Utc;
use github_loco::{
    app::App,
    models::{
        personal_access_tokens::{self, ActiveModel, Entity, TOKEN_SECRET_LEN},
        users::{self, RegisterParams},
    },
};
use loco_rs::testing::prelude::*;
use sea_orm::EntityTrait;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn can_issue_and_authenticate_a_token() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    let user = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "token-owner@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Token Owner".to_owned(),
            username: Some("token-owner".to_owned()),
        },
    )
    .await
    .unwrap();

    let (stored, plaintext) = ActiveModel::issue(
        db,
        user.id,
        "laptop",
        Some("repo:read"),
        Some((Utc::now() + chrono::Duration::days(1)).into()),
    )
    .await
    .unwrap();

    assert!(plaintext.starts_with("glp_"));
    assert!(plaintext.len() > TOKEN_SECRET_LEN);
    assert_ne!(stored.token_hash, plaintext, "only a hash may be stored");

    let (found, owner) = Entity::authenticate(db, &plaintext).await.unwrap();
    assert_eq!(found.id, stored.id);
    assert_eq!(owner.id, user.id);
    assert_eq!(owner.owner_handle(), "token-owner");

    let tokens = Entity::find_by_user(db, user.id).await.unwrap();
    assert_eq!(tokens.len(), 1);
}

#[tokio::test]
#[serial]
async fn rejects_malformed_and_expired_tokens() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    let user = users::Model::create_with_password(
        db,
        &RegisterParams {
            email: "expired@example.com".to_owned(),
            password: "secret".to_owned(),
            name: "Expired".to_owned(),
            username: Some("expired".to_owned()),
        },
    )
    .await
    .unwrap();

    let (_stored, plaintext) = ActiveModel::issue(
        db,
        user.id,
        "old",
        None,
        Some((Utc::now() - chrono::Duration::days(1)).into()),
    )
    .await
    .unwrap();

    assert!(Entity::authenticate(db, "not-a-token").await.is_err());
    assert!(Entity::authenticate(db, &plaintext).await.is_err());
}

#[tokio::test]
#[serial]
async fn can_query_personal_access_tokens() {
    let boot = boot_test::<App>().await.unwrap();
    personal_access_tokens::Entity::find()
        .all(&boot.app_context.db)
        .await
        .expect("`personal_access_tokens` should be queryable — entity and migration must agree");
}
