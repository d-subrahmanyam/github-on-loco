//! Request tests for organizations, teams and org-owned repositories.

use axum::http::{HeaderName, HeaderValue};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use github_loco::{
    app::App,
    dtos::organizations::{OrganizationResponse, TeamResponse},
    dtos::repositories::RepoResponse,
};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

use super::{prepare_data, support};

fn basic(login: &str, password: &str) -> (HeaderName, HeaderValue) {
    let encoded = STANDARD.encode(format!("{login}:{password}"));
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Basic {encoded}")).unwrap(),
    )
}

#[tokio::test]
#[serial]
async fn org_management_is_owner_only_and_team_grants_give_git_access() {
    let _root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (owner_key, owner_value) = prepare_data::auth_header(&owner.token);

        let created = request
            .post("/api/organizations")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({
                "handle": "acme",
                "display_name": "Acme Inc.",
                "description": "widgets"
            }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());
        let org: OrganizationResponse = serde_json::from_str(&created.text()).unwrap();
        assert_eq!(org.handle, "acme");
        assert_eq!(org.viewer_role, "owner");

        let member = prepare_data::register_and_login(
            &request,
            &ctx,
            "member@example.com",
            "Member",
            Some("member"),
            "secret",
        )
        .await;
        let (member_key, member_value) = prepare_data::auth_header(&member.token);

        let added = request
            .post("/api/organizations/acme/members")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({ "username": "member", "role": "member" }))
            .await;
        assert_eq!(added.status_code(), 200, "{}", added.text());

        // A plain member can view the org but cannot change its settings.
        let shown = request
            .get("/api/organizations/acme")
            .add_header(member_key.clone(), member_value.clone())
            .await;
        assert_eq!(shown.status_code(), 200, "{}", shown.text());
        let denied = request
            .post("/api/organizations/acme/teams")
            .add_header(member_key.clone(), member_value.clone())
            .json(&json!({ "name": "backend", "description": null }))
            .await;
        assert_eq!(denied.status_code(), 404, "{}", denied.text());

        let team_response = request
            .post("/api/organizations/acme/teams")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({ "name": "backend", "description": null }))
            .await;
        assert_eq!(team_response.status_code(), 200, "{}", team_response.text());
        let team: TeamResponse = serde_json::from_str(&team_response.text()).unwrap();

        let added_to_team = request
            .post(&format!(
                "/api/organizations/acme/teams/{}/members",
                team.id
            ))
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({ "username": "member", "role": "member" }))
            .await;
        assert_eq!(added_to_team.status_code(), 200, "{}", added_to_team.text());

        // Create the repository under the organization, not the creator.
        let repo_response = request
            .post("/api/repositories")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({
                "name": "api",
                "description": null,
                "is_private": true,
                "organization": "acme"
            }))
            .await;
        assert_eq!(repo_response.status_code(), 200, "{}", repo_response.text());
        let repo: RepoResponse = serde_json::from_str(&repo_response.text()).unwrap();
        assert_eq!(repo.owner, "acme");
        assert!(repo.clone_url.contains("/acme/api.git"));

        let bare = _root.bare_repo("acme", "api");
        assert!(bare.join("HEAD").is_file());

        let push_url = "/acme/api.git/info/refs?service=git-receive-pack";
        let member_basic = basic("member", "secret");
        let before = request
            .get(push_url)
            .add_header(member_basic.0.clone(), member_basic.1.clone())
            .await;
        assert_eq!(before.status_code(), 401, "no grant yet");

        let granted = request
            .post(&format!(
                "/api/organizations/acme/teams/{}/repositories",
                team.id
            ))
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&json!({ "repository": "api", "role": "write" }))
            .await;
        assert_eq!(granted.status_code(), 200, "{}", granted.text());

        let after = request
            .get(push_url)
            .add_header(member_basic.0.clone(), member_basic.1.clone())
            .await;
        assert_eq!(after.status_code(), 200, "{}", after.text());

        let overview = request
            .get("/api/repos/acme/api")
            .add_header(member_key, member_value)
            .await;
        let overview: RepoResponse = serde_json::from_str(&overview.text()).unwrap();
        assert_eq!(overview.viewer_role, "write");
        assert_eq!(overview.owner, "acme");

        // A non-member cannot see the private org repository at all.
        let outsider = prepare_data::register_and_login(
            &request,
            &ctx,
            "outsider@example.com",
            "Outsider",
            Some("outsider"),
            "secret",
        )
        .await;
        let (outsider_key, outsider_value) = prepare_data::auth_header(&outsider.token);
        let hidden = request
            .get("/api/repos/acme/api")
            .add_header(outsider_key, outsider_value)
            .await;
        assert_eq!(hidden.status_code(), 404);

        // Revoking the grant removes the git access immediately.
        let revoked = request
            .delete(&format!(
                "/api/organizations/acme/teams/{}/repositories/{}",
                team.id, repo.id
            ))
            .add_header(owner_key, owner_value)
            .await;
        assert_eq!(revoked.status_code(), 200, "{}", revoked.text());
        let gone = request
            .get(push_url)
            .add_header(member_basic.0, member_basic.1)
            .await;
        assert_eq!(gone.status_code(), 401, "grant was revoked");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn org_and_personal_repos_with_the_same_name_do_not_shadow_each_other() {
    let _root = support::TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let owner = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&owner.token);

        let org = request
            .post("/api/organizations")
            .add_header(key.clone(), value.clone())
            .json(&json!({ "handle": "acme", "display_name": "Acme" }))
            .await;
        assert_eq!(org.status_code(), 200, "{}", org.text());

        for organization in [json!("acme"), json!(null)] {
            let body = json!({
                "name": "api",
                "description": null,
                "is_private": false,
                "organization": organization
            });
            let response = request
                .post("/api/repositories")
                .add_header(key.clone(), value.clone())
                .json(&body)
                .await;
            assert_eq!(response.status_code(), 200, "{}", response.text());
        }

        let org_repo = request.get("/api/repos/acme/api").await;
        let org_repo: RepoResponse = serde_json::from_str(&org_repo.text()).unwrap();
        assert!(org_repo.clone_url.contains("/acme/api.git"));

        let personal = request.get("/api/repos/loco/api").await;
        let personal: RepoResponse = serde_json::from_str(&personal.text()).unwrap();
        assert!(personal.clone_url.contains("/loco/api.git"));
        assert_ne!(org_repo.id, personal.id);
    })
    .await;
}
