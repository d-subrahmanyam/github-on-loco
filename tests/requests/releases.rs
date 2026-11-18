//! Request tests for releases, tags and source archives.

use github_loco::{
    app::App,
    dtos::releases::{ReleaseResponse, TagResponse},
};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;
use std::process::Command;

use super::{prepare_data, support::TestRepoRoot};

fn git_tag_names(bare: &std::path::Path) -> Vec<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(bare)
        .args(["tag", "-l"])
        .output()
        .expect("git should be installed");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

#[tokio::test]
#[serial]
async fn releases_manage_git_tags_and_archives() {
    let root = TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&owner.token);
        let repo_url = "/api/repos/loco/app";

        let created = request
            .post("/api/repositories")
            .add_header(key.clone(), value.clone())
            .json(&json!({ "name": "app", "is_private": false }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());

        let bare = root.bare_repo("loco", "app");
        super::support::seed_commit(&bare);

        let body = json!({
            "tag_name": "v1.0.0",
            "target": "main",
            "title": "Version 1.0.0",
            "body": "first stable release",
            "is_prerelease": false
        });
        let published = request
            .post(&format!("{repo_url}/releases"))
            .add_header(key.clone(), value.clone())
            .json(&body)
            .await;
        assert_eq!(published.status_code(), 200, "{}", published.text());
        let release: ReleaseResponse = serde_json::from_str(&published.text()).unwrap();
        assert_eq!(release.tag_name, "v1.0.0");
        assert_eq!(release.author, "loco");
        assert!(!release.commit_sha.is_empty());
        assert_eq!(git_tag_names(&bare), vec!["v1.0.0"]);

        let listed = request.get(&format!("{repo_url}/releases")).await;
        let listed: Vec<ReleaseResponse> = serde_json::from_str(&listed.text()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, release.id);

        let latest = request.get(&format!("{repo_url}/releases/latest")).await;
        let latest: ReleaseResponse = serde_json::from_str(&latest.text()).unwrap();
        assert_eq!(latest.id, release.id);

        let shown = request
            .get(&format!("{repo_url}/releases/{}", release.id))
            .await;
        let shown: ReleaseResponse = serde_json::from_str(&shown.text()).unwrap();
        assert_eq!(shown.body.as_deref(), Some("first stable release"));

        let tags = request.get(&format!("{repo_url}/tags")).await;
        let tags: Vec<TagResponse> = serde_json::from_str(&tags.text()).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v1.0.0");
        assert_eq!(tags[0].sha, release.commit_sha);

        let archive = request
            .get(&format!("{repo_url}/archive/v1.0.0.tar.gz"))
            .await;
        assert_eq!(archive.status_code(), 200, "{}", archive.text());
        assert!(archive
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("gzip")));
        let bytes = archive.text().into_bytes();
        // `text()` is lossy for binary data, but the gzip magic bytes survive
        // ASCII decoding.
        assert_eq!(bytes.first(), Some(&0x1f));

        // A second release on the same tag cannot move the published tag.
        let duplicate = request
            .post(&format!("{repo_url}/releases"))
            .add_header(key.clone(), value.clone())
            .json(&body)
            .await;
        assert_eq!(duplicate.status_code(), 400, "{}", duplicate.text());
        assert_eq!(git_tag_names(&bare), vec!["v1.0.0"]);

        let updated = request
            .patch(&format!("{repo_url}/releases/{}", release.id))
            .add_header(key.clone(), value.clone())
            .json(&json!({
                "title": "Version 1.0.0 (updated)",
                "body": "revised notes",
                "is_prerelease": true
            }))
            .await;
        assert_eq!(updated.status_code(), 200, "{}", updated.text());
        let updated: ReleaseResponse = serde_json::from_str(&updated.text()).unwrap();
        assert_eq!(updated.title, "Version 1.0.0 (updated)");
        assert!(updated.is_prerelease);
        let latest = request.get(&format!("{repo_url}/releases/latest")).await;
        assert_eq!(latest.status_code(), 404, "only a prerelease is left");

        let removed = request
            .delete(&format!("{repo_url}/releases/{}", release.id))
            .add_header(key.clone(), value.clone())
            .await;
        assert_eq!(removed.status_code(), 200, "{}", removed.text());
        assert!(git_tag_names(&bare).is_empty());
        assert!(request
            .get(&format!("{repo_url}/releases"))
            .await
            .text()
            .contains("[]"));

        // A loose tag can be pushed and deleted without a release.
        Command::new("git")
            .arg("-C")
            .arg(&bare)
            .args(["tag", "v0.9.0"])
            .status()
            .unwrap();
        let deleted_tag = request
            .delete(&format!("{repo_url}/tags/v0.9.0"))
            .add_header(key, value)
            .await;
        assert_eq!(deleted_tag.status_code(), 200, "{}", deleted_tag.text());
        assert!(git_tag_names(&bare).is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn release_endpoints_respect_read_and_write_permissions() {
    let _root = TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&owner.token);
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

        for (name, is_private) in [("public-app", false), ("private-app", true)] {
            let created = request
                .post("/api/repositories")
                .add_header(key.clone(), value.clone())
                .json(&json!({ "name": name, "is_private": is_private }))
                .await;
            assert_eq!(created.status_code(), 200, "{}", created.text());
        }

        let body = json!({
            "tag_name": "v1",
            "target": "HEAD",
            "title": "First",
            "body": null,
            "is_prerelease": false
        });

        // A stranger cannot publish a release on either repository.
        for repo in ["public-app", "private-app"] {
            let denied = request
                .post(&format!("/api/repos/loco/{repo}/releases"))
                .add_header(stranger_key.clone(), stranger_value.clone())
                .json(&body)
                .await;
            assert_eq!(denied.status_code(), 404, "{}", denied.text());
        }

        // The owner can release even on an empty repository (HEAD resolves to
        // a bare repo with no commits? no — git refuses, so seed one first).
        let bare = _root.bare_repo("loco", "public-app");
        super::support::seed_commit(&bare);
        let allowed = request
            .post("/api/repos/loco/public-app/releases")
            .add_header(key.clone(), value.clone())
            .json(&json!({
                "tag_name": "v1",
                "target": "main",
                "title": "First",
                "body": null,
                "is_prerelease": false
            }))
            .await;
        assert_eq!(allowed.status_code(), 200, "{}", allowed.text());

        // Public releases are readable anonymously; private ones are 404.
        let public = request.get("/api/repos/loco/public-app/releases").await;
        assert_eq!(public.status_code(), 200);
        let private = request.get("/api/repos/loco/private-app/releases").await;
        assert_eq!(private.status_code(), 404);
        let private_as_stranger = request
            .get("/api/repos/loco/private-app/releases")
            .add_header(stranger_key, stranger_value)
            .await;
        assert_eq!(private_as_stranger.status_code(), 404);
    })
    .await;
}
