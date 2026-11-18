//! Request tests for stars, watches and the activity feed.

use github_loco::{
    app::App,
    dtos::repositories::RepoResponse,
    dtos::social::{EventResponse, RepoStatsResponse},
};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

use super::{prepare_data, support};

async fn create_public_repo(
    request: &loco_rs::TestServer,
    key: &axum::http::HeaderName,
    value: &axum::http::HeaderValue,
) {
    let response = request
        .post("/api/repositories")
        .add_header(key.clone(), value.clone())
        .json(&json!({ "name": "social", "is_private": false }))
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
}

#[tokio::test]
#[serial]
async fn stars_and_watches_update_counts_and_the_feed() {
    let _root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (owner_key, owner_value) = prepare_data::auth_header(&owner.token);
        create_public_repo(&request, &owner_key, &owner_value).await;

        let guest = prepare_data::register_and_login(
            &request,
            &ctx,
            "guest@example.com",
            "Guest",
            Some("guest"),
            "secret",
        )
        .await;
        let (key, value) = prepare_data::auth_header(&guest.token);
        let base = "/api/repos/loco/social";

        let overview = request
            .get(base)
            .add_header(key.clone(), value.clone())
            .await;
        let overview: RepoResponse = serde_json::from_str(&overview.text()).unwrap();
        assert_eq!(overview.star_count, 0);
        assert!(!overview.starred);

        let starred = request
            .post(&format!("{base}/star"))
            .add_header(key.clone(), value.clone())
            .await;
        assert_eq!(starred.status_code(), 200, "{}", starred.text());
        let stats: RepoStatsResponse = serde_json::from_str(&starred.text()).unwrap();
        assert_eq!(stats.star_count, 1);
        assert!(stats.starred);

        // A repeat star is idempotent.
        let again = request
            .post(&format!("{base}/star"))
            .add_header(key.clone(), value.clone())
            .await;
        let stats: RepoStatsResponse = serde_json::from_str(&again.text()).unwrap();
        assert_eq!(stats.star_count, 1);

        let watched = request
            .post(&format!("{base}/watch"))
            .add_header(key.clone(), value.clone())
            .await;
        assert_eq!(watched.status_code(), 200, "{}", watched.text());
        let stats: RepoStatsResponse = serde_json::from_str(&watched.text()).unwrap();
        assert_eq!(stats.watch_count, 1);
        assert!(stats.watching);

        let overview = request
            .get(base)
            .add_header(key.clone(), value.clone())
            .await;
        let overview: RepoResponse = serde_json::from_str(&overview.text()).unwrap();
        assert_eq!(overview.star_count, 1);
        assert_eq!(overview.watch_count, 1);
        assert!(overview.starred && overview.watching);

        let feed = request.get(&format!("{base}/events")).await;
        assert_eq!(feed.status_code(), 200, "{}", feed.text());
        let events: Vec<EventResponse> = serde_json::from_str(&feed.text()).unwrap();
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|event| event.actor == "guest"));
        assert_eq!(events[0].kind, "watch");
        assert_eq!(events[0].text, "started watching this repository");
        assert_eq!(events[1].kind, "star");

        let unstarred = request
            .delete(&format!("{base}/star"))
            .add_header(key.clone(), value.clone())
            .await;
        let stats: RepoStatsResponse = serde_json::from_str(&unstarred.text()).unwrap();
        assert_eq!(stats.star_count, 0);
        assert!(!stats.starred);

        let unwatched = request
            .delete(&format!("{base}/watch"))
            .add_header(key, value)
            .await;
        let stats: RepoStatsResponse = serde_json::from_str(&unwatched.text()).unwrap();
        assert_eq!(stats.watch_count, 0);
        assert!(!stats.watching);

        let feed = request.get(&format!("{base}/events")).await;
        let events: Vec<EventResponse> = serde_json::from_str(&feed.text()).unwrap();
        assert_eq!(events[0].kind, "unwatch");
        assert_eq!(events[1].kind, "unstar");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn social_actions_respect_authentication_and_visibility() {
    let _root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (owner_key, owner_value) = prepare_data::auth_header(&owner.token);
        let created = request
            .post("/api/repositories")
            .add_header(owner_key, owner_value)
            .json(&json!({ "name": "hidden", "is_private": true }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());

        let base = "/api/repos/loco/hidden";
        assert_eq!(
            request.post(&format!("{base}/star")).await.status_code(),
            401
        );
        assert_eq!(
            request.get(&format!("{base}/events")).await.status_code(),
            404
        );

        let guest = prepare_data::register_and_login(
            &request,
            &ctx,
            "guest@example.com",
            "Guest",
            Some("guest"),
            "secret",
        )
        .await;
        let (key, value) = prepare_data::auth_header(&guest.token);
        assert_eq!(
            request
                .get(&format!("{base}/events"))
                .add_header(key, value)
                .await
                .status_code(),
            404
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn issues_and_pull_requests_appear_in_the_feed() {
    let root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&owner.token);
        let created = request
            .post("/api/repositories")
            .add_header(key.clone(), value.clone())
            .json(&json!({ "name": "feed", "is_private": false }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());

        let bare = root.bare_repo("loco", "feed");
        support::seed_commit(&bare);
        support::seed_feature_branch(&bare, "feature", "feature.txt", "feature\n");

        let issue = request
            .post("/api/repos/loco/feed/issues")
            .add_header(key.clone(), value.clone())
            .json(&json!({ "title": "A bug", "body": null, "label_ids": [] }))
            .await;
        assert_eq!(issue.status_code(), 200, "{}", issue.text());

        let pr = request
            .post("/api/repos/loco/feed/pulls")
            .add_header(key.clone(), value.clone())
            .json(&json!({
                "title": "A change",
                "body": null,
                "head_ref": "feature",
                "base_ref": "main"
            }))
            .await;
        assert_eq!(pr.status_code(), 200, "{}", pr.text());

        let merged = request
            .post("/api/repos/loco/feed/pulls/1/merge")
            .add_header(key, value)
            .await;
        assert_eq!(merged.status_code(), 200, "{}", merged.text());

        let feed = request.get("/api/repos/loco/feed/events").await;
        let events: Vec<EventResponse> = serde_json::from_str(&feed.text()).unwrap();
        let kinds: Vec<&str> = events.iter().map(|event| event.kind.as_str()).collect();
        assert!(kinds.contains(&"pr.merged"), "{kinds:?}");
        assert!(kinds.contains(&"pr.opened"), "{kinds:?}");
        assert!(kinds.contains(&"issue.opened"), "{kinds:?}");
        let merged_event = events
            .iter()
            .find(|event| event.kind == "pr.merged")
            .unwrap();
        assert_eq!(merged_event.text, "merged pull request #1");
    })
    .await;
}
