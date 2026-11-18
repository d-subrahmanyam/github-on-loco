//! Wire types for organizations, teams and their grants.

use ts_rs::TS;

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct OrganizationResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub handle: String,
    pub display_name: String,
    pub description: Option<String>,
    /// `owner`, `member` or `none` for the requesting user.
    pub viewer_role: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateOrganizationParams {
    pub handle: String,
    pub display_name: String,
    pub description: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct OrgMemberResponse {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
    pub role: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct AddOrgMemberParams {
    pub username: String,
    pub role: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct TeamResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    #[ts(type = "number")]
    pub member_count: i64,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateTeamParams {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct TeamMemberResponse {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct TeamRepoResponse {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub team_id: i64,
    pub owner: String,
    pub name: String,
    pub role: String,
}

/// `repository` is an `owner/name` handle within this organization.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct GrantTeamRepoParams {
    pub repository: String,
    pub role: String,
}
