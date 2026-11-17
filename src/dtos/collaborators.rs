//! Wire types for repository collaborator management.

use ts_rs::TS;

/// Body of `POST /api/repos/{owner}/{repo}/collaborators`.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct AddCollaboratorParams {
    /// Public handle of the user to grant access to.
    pub username: String,
    /// `read`, `write` or `admin`.
    pub role: String,
}

/// Body of `PATCH /api/repos/{owner}/{repo}/collaborators/{user_id}`.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct UpdateCollaboratorParams {
    pub role: String,
}

/// One collaborator as shown in the SPA.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CollaboratorResponse {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub user_id: i64,
    pub username: String,
    pub role: String,
}
