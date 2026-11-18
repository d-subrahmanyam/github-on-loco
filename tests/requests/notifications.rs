//! Feature-flag and fan-out tests for notifications.

use github_loco::{
    app::App,
    dtos::notifications::{NotificationResponse, NotificationSettingsResponse},
};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

use super::{prepare_data, support::TestRepoRoot};

/// Sets the flag for the duration of a `#[serial]` test.
struct FlagGuard;

impl FlagGuard {
    fn on() -> Self {
        std::env::set_var("NOTIFICATIONS_ENABLED", "true");
        Self
    }
}

impl Drop for FlagGuard {
    fn drop(&mut self) {
        std::env::remove_var("NOTIFICATIONS_ENABLED");
    }
}

async fn star_while_watched(request: &loco_rs::TestServer, ctx: &loco_rs::app::AppContext) {
    let owner = prepare_data::init_user_login(request, ctx).await;
    let (owner_key, owner_value) = prepare_data::auth_header(&owner.token);
    let created = request
        .post("/api/repositories")
        .add_header(owner_key, owner_value)
        .json(&json!({ "name": "app", "is_private": false }))
        .await;
    assert_eq!(created.status_code(), 200, "{}", created.text());

    let watcher = prepare_data::register_and_login(
        request,
        ctx,
        "watcher@example.com",
        "Watcher",
        Some("watcher"),
        "secret",
    )
    .await;
    let actor = prepare_data::register_and_login(
        request,
        ctx,
        "actor@example.com",
        "Actor",
        Some("actor"),
        "secret",
    )
    .await;
    let (watcher_key, watcher_value) = prepare_data::auth_header(&watcher.token);
    let (actor_key, actor_value) = prepare_data::auth_header(&actor.token);

    let watched = request
        .post("/api/repos/loco/app/watch")
        .add_header(watcher_key, watcher_value)
        .await;
    assert_eq!(watched.status_code(), 200, "{}", watched.text());
    let starred = request
        .post("/api/repos/loco/app/star")
        .add_header(actor_key, actor_value)
        .await;
    assert_eq!(starred.status_code(), 200, "{}", starred.text());
}

#[tokio::test]
#[serial]
async fn notifications_are_off_by_default() {
    std::env::remove_var("NOTIFICATIONS_ENABLED");
    let _root = TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        star_while_watched(&request, &ctx).await;
        // Log in as the existing watcher rather than registering a duplicate.
        let login = request
            .post("/api/auth/login")
            .json(&json!({ "email": "watcher@example.com", "password": "secret" }))
            .await;
        let token: serde_json::Value = serde_json::from_str(&login.text()).unwrap();
        let (key, value) = prepare_data::auth_header(token["token"].as_str().unwrap());

        let settings = request
            .get("/api/notifications/settings")
            .add_header(key.clone(), value.clone())
            .await;
        let settings: NotificationSettingsResponse =
            serde_json::from_str(&settings.text()).unwrap();
        assert!(!settings.enabled);
        assert!(settings.available.contains(&"log".to_owned()));
        assert!(settings.available.contains(&"email".to_owned()));

        let list = request
            .get("/api/notifications")
            .add_header(key, value)
            .await;
        assert_eq!(list.text(), "[]", "no rows when the flag is off");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn enabled_notifications_fan_out_to_watchers_but_not_the_actor() {
    let _guard = FlagGuard::on();
    let _root = TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        star_while_watched(&request, &ctx).await;

        let watcher_login = request
            .post("/api/auth/login")
            .json(&json!({ "email": "watcher@example.com", "password": "secret" }))
            .await;
        let watcher_token: serde_json::Value = serde_json::from_str(&watcher_login.text()).unwrap();
        let (watcher_key, watcher_value) =
            prepare_data::auth_header(watcher_token["token"].as_str().unwrap());

        let settings = request
            .get("/api/notifications/settings")
            .add_header(watcher_key.clone(), watcher_value.clone())
            .await;
        let settings: NotificationSettingsResponse =
            serde_json::from_str(&settings.text()).unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.channels, vec!["log".to_owned()]);

        let list = request
            .get("/api/notifications")
            .add_header(watcher_key, watcher_value)
            .await;
        let list: Vec<NotificationResponse> = serde_json::from_str(&list.text()).unwrap();
        assert_eq!(
            list.len(),
            1,
            "exactly one log notification for the watcher"
        );
        assert_eq!(list[0].event, "star");
        assert_eq!(list[0].channel, "log");
        assert_eq!(list[0].status, "sent");
        assert_eq!(list[0].attempts, 1);
        assert!(list[0].title.contains("loco/app"));

        let actor_login = request
            .post("/api/auth/login")
            .json(&json!({ "email": "actor@example.com", "password": "secret" }))
            .await;
        let actor_token: serde_json::Value = serde_json::from_str(&actor_login.text()).unwrap();
        let (actor_key, actor_value) =
            prepare_data::auth_header(actor_token["token"].as_str().unwrap());
        let actor_list = request
            .get("/api/notifications")
            .add_header(actor_key, actor_value)
            .await;
        assert_eq!(actor_list.text(), "[]", "the actor is never notified");
    })
    .await;
}
