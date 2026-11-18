//! Organization and team management. Owners manage membership and grants;
//! members may browse. Authorization stays in `repositories::Model::can` for
//! repositories — this controller only guards the org's own settings.

#![allow(clippy::unused_async)]

use loco_rs::prelude::*;

use crate::dtos::organizations::{
    AddOrgMemberParams, CreateOrganizationParams, CreateTeamParams, GrantTeamRepoParams,
    OrgMemberResponse, OrganizationResponse, TeamMemberResponse, TeamRepoResponse, TeamResponse,
};
use crate::models::{
    org_members, organizations, repositories, team_members, team_repositories, teams, users,
};

async fn current_user(ctx: &AppContext, auth: &auth::JWT) -> Result<users::Model> {
    Ok(users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?)
}

async fn load_org(ctx: &AppContext, handle: &str) -> Result<organizations::Model> {
    organizations::Entity::find_by_handle(&ctx.db, handle)
        .await
        .map_err(Into::into)
}

pub(crate) async fn require_member(
    ctx: &AppContext,
    org: &organizations::Model,
    user_id: i64,
) -> Result<()> {
    if org_members::Entity::find_for(&ctx.db, org.id, user_id)
        .await?
        .is_none()
    {
        return not_found();
    }
    Ok(())
}

async fn require_owner(ctx: &AppContext, org: &organizations::Model, user_id: i64) -> Result<()> {
    if !org.is_owner(&ctx.db, user_id).await? {
        return not_found();
    }
    Ok(())
}

fn org_response(org: &organizations::Model, viewer_role: String) -> OrganizationResponse {
    OrganizationResponse {
        id: org.id,
        handle: org.handle.clone(),
        display_name: org.display_name.clone(),
        description: org.description.clone(),
        viewer_role,
    }
}

/// `POST /api/organizations`.
#[debug_handler]
pub async fn create(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateOrganizationParams>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = organizations::ActiveModel::create_with_owner(
        &ctx.db,
        user.id,
        &params.handle,
        &params.display_name,
        params.description.as_deref(),
    )
    .await?;
    format::json(org_response(&org, organizations::ROLE_OWNER.to_owned()))
}

/// `GET /api/organizations` — organizations the current user belongs to.
#[debug_handler]
pub async fn list(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let orgs = organizations::Entity::find_by_user(&ctx.db, user.id).await?;
    let mut response = Vec::with_capacity(orgs.len());
    for org in orgs {
        let role = org.role_for(&ctx.db, user.id).await?.unwrap_or_default();
        response.push(org_response(&org, role));
    }
    format::json(response)
}

/// `GET /api/organizations/{handle}`.
#[debug_handler]
pub async fn show(
    auth: auth::JWT,
    Path(handle): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = load_org(&ctx, &handle).await?;
    require_member(&ctx, &org, user.id).await?;
    let role = org.role_for(&ctx.db, user.id).await?.unwrap_or_default();
    format::json(org_response(&org, role))
}

/// `GET /api/organizations/{handle}/members`.
#[debug_handler]
pub async fn members(
    auth: auth::JWT,
    Path(handle): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = load_org(&ctx, &handle).await?;
    require_member(&ctx, &org, user.id).await?;
    let rows = org_members::Entity::find_by_organization(&ctx.db, org.id).await?;
    let mut response = Vec::with_capacity(rows.len());
    for row in rows {
        response.push(OrgMemberResponse {
            id: row.id,
            user_id: row.user_id,
            username: organizations::member_username(&ctx.db, row.user_id).await?,
            role: row.role,
        });
    }
    format::json(response)
}

/// `POST /api/organizations/{handle}/members`.
#[debug_handler]
pub async fn add_member(
    auth: auth::JWT,
    Path(handle): Path<String>,
    State(ctx): State<AppContext>,
    Json(params): Json<AddOrgMemberParams>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = load_org(&ctx, &handle).await?;
    require_owner(&ctx, &org, user.id).await?;
    let invited = users::Model::find_by_username(&ctx.db, &params.username).await?;
    let row = org_members::ActiveModel::add(&ctx.db, org.id, invited.id, &params.role).await?;
    format::json(OrgMemberResponse {
        id: row.id,
        user_id: invited.id,
        username: invited.username,
        role: row.role,
    })
}

/// `DELETE /api/organizations/{handle}/members/{user_id}`.
#[debug_handler]
pub async fn remove_member(
    auth: auth::JWT,
    Path((handle, user_id)): Path<(String, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = load_org(&ctx, &handle).await?;
    require_owner(&ctx, &org, user.id).await?;
    org_members::ActiveModel::remove(&ctx.db, org.id, user_id).await?;
    format::empty_json()
}

/// `GET /api/organizations/{handle}/teams`.
#[debug_handler]
pub async fn list_teams(
    auth: auth::JWT,
    Path(handle): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = load_org(&ctx, &handle).await?;
    require_member(&ctx, &org, user.id).await?;
    let rows = teams::Entity::find_by_organization(&ctx.db, org.id).await?;
    let mut response = Vec::with_capacity(rows.len());
    for team in rows {
        response.push(TeamResponse {
            id: team.id,
            name: team.name,
            description: team.description,
            member_count: team_members::Entity::count_for_team(&ctx.db, team.id).await?,
        });
    }
    format::json(response)
}

/// `POST /api/organizations/{handle}/teams`.
#[debug_handler]
pub async fn create_team(
    auth: auth::JWT,
    Path(handle): Path<String>,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateTeamParams>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = load_org(&ctx, &handle).await?;
    require_owner(&ctx, &org, user.id).await?;
    let team =
        teams::ActiveModel::create(&ctx.db, org.id, &params.name, params.description.as_deref())
            .await?;
    format::json(TeamResponse {
        id: team.id,
        name: team.name,
        description: team.description,
        member_count: 0,
    })
}

async fn load_team(
    ctx: &AppContext,
    org: &organizations::Model,
    team_id: i64,
) -> Result<teams::Model> {
    teams::Entity::find_in_organization(&ctx.db, org.id, team_id)
        .await
        .map_err(Into::into)
}

/// `GET /api/organizations/{handle}/teams/{team_id}/members`.
#[debug_handler]
pub async fn team_members_list(
    auth: auth::JWT,
    Path((handle, team_id)): Path<(String, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = load_org(&ctx, &handle).await?;
    require_member(&ctx, &org, user.id).await?;
    let team = load_team(&ctx, &org, team_id).await?;
    let rows = team_members::Entity::find_by_team(&ctx.db, team.id).await?;
    let mut response = Vec::with_capacity(rows.len());
    for row in rows {
        response.push(TeamMemberResponse {
            id: row.id,
            user_id: row.user_id,
            username: organizations::member_username(&ctx.db, row.user_id).await?,
        });
    }
    format::json(response)
}

/// `POST /api/organizations/{handle}/teams/{team_id}/members`.
#[debug_handler]
pub async fn add_team_member(
    auth: auth::JWT,
    Path((handle, team_id)): Path<(String, i64)>,
    State(ctx): State<AppContext>,
    Json(params): Json<AddOrgMemberParams>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = load_org(&ctx, &handle).await?;
    require_owner(&ctx, &org, user.id).await?;
    let team = load_team(&ctx, &org, team_id).await?;
    let member = users::Model::find_by_username(&ctx.db, &params.username).await?;
    let row = team_members::ActiveModel::add(&ctx.db, &team, member.id).await?;
    format::json(TeamMemberResponse {
        id: row.id,
        user_id: member.id,
        username: member.username,
    })
}

/// `DELETE /api/organizations/{handle}/teams/{team_id}/members/{user_id}`.
#[debug_handler]
pub async fn remove_team_member(
    auth: auth::JWT,
    Path((handle, team_id, user_id)): Path<(String, i64, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = load_org(&ctx, &handle).await?;
    require_owner(&ctx, &org, user.id).await?;
    let team = load_team(&ctx, &org, team_id).await?;
    team_members::ActiveModel::remove(&ctx.db, team.id, user_id).await?;
    format::empty_json()
}

/// `GET /api/organizations/{handle}/teams/{team_id}/repositories`.
#[debug_handler]
pub async fn team_repos(
    auth: auth::JWT,
    Path((handle, team_id)): Path<(String, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = load_org(&ctx, &handle).await?;
    require_member(&ctx, &org, user.id).await?;
    let team = load_team(&ctx, &org, team_id).await?;
    let grants = team_repositories::Entity::find_by_team(&ctx.db, team.id).await?;
    let mut response = Vec::with_capacity(grants.len());
    for grant in grants {
        let Some(repo) = repositories::Entity::find_by_id(grant.repository_id)
            .one(&ctx.db)
            .await?
        else {
            continue;
        };
        response.push(TeamRepoResponse {
            id: grant.id,
            team_id: team.id,
            owner: org.handle.clone(),
            name: repo.display_name()?.to_owned(),
            role: grant.role,
        });
    }
    format::json(response)
}

/// `POST /api/organizations/{handle}/teams/{team_id}/repositories`.
#[debug_handler]
pub async fn grant_repo(
    auth: auth::JWT,
    Path((handle, team_id)): Path<(String, i64)>,
    State(ctx): State<AppContext>,
    Json(params): Json<GrantTeamRepoParams>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = load_org(&ctx, &handle).await?;
    require_owner(&ctx, &org, user.id).await?;
    let team = load_team(&ctx, &org, team_id).await?;
    // The handle must belong to this org; never let a team reach outside it.
    let repo =
        repositories::Model::find_by_owner_and_name(&ctx.db, &org.handle, &params.repository)
            .await?;
    let grant =
        team_repositories::ActiveModel::grant(&ctx.db, &team, repo.id, &params.role).await?;
    format::json(TeamRepoResponse {
        id: grant.id,
        team_id: team.id,
        owner: org.handle.clone(),
        name: repo.display_name()?.to_owned(),
        role: grant.role,
    })
}

/// `DELETE /api/organizations/{handle}/teams/{team_id}/repositories/{repository_id}`.
#[debug_handler]
pub async fn revoke_repo(
    auth: auth::JWT,
    Path((handle, team_id, repository_id)): Path<(String, i64, i64)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = current_user(&ctx, &auth).await?;
    let org = load_org(&ctx, &handle).await?;
    require_owner(&ctx, &org, user.id).await?;
    let team = load_team(&ctx, &org, team_id).await?;
    team_repositories::ActiveModel::revoke(&ctx.db, team.id, repository_id).await?;
    format::empty_json()
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/organizations")
        .add("/", post(create))
        .add("/", get(list))
        .add("/{handle}", get(show))
        .add("/{handle}/members", get(members))
        .add("/{handle}/members", post(add_member))
        .add("/{handle}/members/{user_id}", delete(remove_member))
        .add("/{handle}/teams", get(list_teams))
        .add("/{handle}/teams", post(create_team))
        .add("/{handle}/teams/{team_id}/members", get(team_members_list))
        .add("/{handle}/teams/{team_id}/members", post(add_team_member))
        .add(
            "/{handle}/teams/{team_id}/members/{user_id}",
            delete(remove_team_member),
        )
        .add("/{handle}/teams/{team_id}/repositories", get(team_repos))
        .add("/{handle}/teams/{team_id}/repositories", post(grant_repo))
        .add(
            "/{handle}/teams/{team_id}/repositories/{repository_id}",
            delete(revoke_repo),
        )
}
