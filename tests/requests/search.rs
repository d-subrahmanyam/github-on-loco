//! Request tests for global search visibility and case-insensitive matching.

use github_loco::{
    app::App,
    dtos::{repositories::RepoResponse, search::SearchResult},
    models::{issues, pull_requests},
};
use loco_rs::testing::prelude::*;
use sea_orm::EntityTrait;
use serde_json::json;
use serial_test::serial;

use super::{prepare_data, support::TestRepoRoot};

#[tokio::test]
#[serial]
async fn search_respects_private_repository_visibility() {
    let _root = TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&owner.token);

        let mut ids = std::collections::HashMap::new();
        for (name, description, is_private) in [
            ("alpha", "A widget library", false),
            ("secret-box", "Hidden widget internals", true),
        ] {
            let created = request
                .post("/api/repositories")
                .add_header(key.clone(), value.clone())
                .json(&json!({
                    "name": name,
                    "description": description,
                    "is_private": is_private
                }))
                .await;
            assert_eq!(created.status_code(), 200, "{}", created.text());
            let repo: RepoResponse = serde_json::from_str(&created.text()).unwrap();
            ids.insert(name, repo.id);
        }

        let alpha = ids["alpha"];
        let secret = ids["secret-box"];
        issues::ActiveModel::create_for_repository(
            &ctx.db,
            alpha,
            owner.user.id,
            "Widget crash on startup",
            Some("the widget panics"),
            &[],
        )
        .await
        .unwrap();
        issues::ActiveModel::create_for_repository(
            &ctx.db,
            secret,
            owner.user.id,
            "Widget leak in private repo",
            None,
            &[],
        )
        .await
        .unwrap();
        pull_requests::ActiveModel::create_for_repository(
            &ctx.db,
            alpha,
            owner.user.id,
            "Fix widget crash",
            Some("patch"),
            "fix",
            "abc123",
            "main",
        )
        .await
        .unwrap();

        // Anonymous search sees only the public repository and its contents.
        let anonymous = request.get("/api/search?q=widget").await;
        assert_eq!(anonymous.status_code(), 200, "{}", anonymous.text());
        let result: SearchResult = serde_json::from_str(&anonymous.text()).unwrap();
        assert_eq!(result.repositories.len(), 1);
        assert_eq!(result.repositories[0].name, "alpha");
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].title, "Widget crash on startup");
        assert_eq!(result.issues[0].owner, "loco");
        assert_eq!(result.pulls.len(), 1);
        assert_eq!(result.pulls[0].title, "Fix widget crash");
        assert_eq!(result.total, 3);

        // Case-insensitive, and a short query is rejected.
        let upper = request.get("/api/search?q=WIDGET").await;
        let upper: SearchResult = serde_json::from_str(&upper.text()).unwrap();
        assert_eq!(upper.repositories.len(), 1);
        let short = request.get("/api/search?q=w").await;
        assert_eq!(short.status_code(), 400, "{}", short.text());

        // The owner sees the private repository and its issue too.
        let authed = request
            .get("/api/search?q=widget")
            .add_header(key, value)
            .await;
        let authed: SearchResult = serde_json::from_str(&authed.text()).unwrap();
        assert_eq!(authed.repositories.len(), 2);
        assert_eq!(authed.issues.len(), 2);
        assert!(authed
            .issues
            .iter()
            .any(|hit| hit.repo == "secret-box" && hit.title.contains("Widget leak")));

        // A stranger with no grant still cannot see the private repo.
        let stranger = prepare_data::register_and_login(
            &request,
            &ctx,
            "stranger@example.com",
            "Stranger",
            Some("stranger"),
            "secret",
        )
        .await;
        let (stranger_key, stranger_value) = prepare_data::auth_header(&stranger.token);
        let as_stranger = request
            .get("/api/search?q=widget")
            .add_header(stranger_key, stranger_value)
            .await;
        let as_stranger: SearchResult = serde_json::from_str(&as_stranger.text()).unwrap();
        assert_eq!(as_stranger.repositories.len(), 1);
        assert_eq!(as_stranger.issues.len(), 1);
        assert_eq!(
            issues::Entity::find().all(&ctx.db).await.unwrap().len(),
            2,
            "search must not mutate data"
        );
    })
    .await;
}
