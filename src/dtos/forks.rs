//! Wire types for the fork API.

use ts_rs::TS;

/// Body of `POST /api/repos/{owner}/{repo}/fork`. Omit `name` to keep the
/// source repository's name (the server picks a free `-fork-N` variant if it is
/// already taken by the current user).
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateForkParams {
    pub name: Option<String>,
}

/// The source a fork was copied from, shown as "forked from owner/name".
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct ForkParentResponse {
    pub owner: String,
    pub name: String,
}

/// One fork in a source repository's fork list.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct ForkResponse {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub repository_id: i64,
    pub owner: String,
    pub name: String,
    pub created_at: String,
}
