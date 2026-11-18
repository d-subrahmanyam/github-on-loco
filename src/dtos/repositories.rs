//! Wire types for the repository API.
//!
//! Entities never go on the wire directly — `users::Model` carries a password
//! hash and both entities carry internal ids. These DTOs are also exported to
//! TypeScript so the SPA in `frontend/` consumes exactly the same contract.

use ts_rs::TS;

/// Body of `POST /api/repositories`.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateRepoParams {
    pub name: String,
    pub description: Option<String>,
    pub is_private: bool,
}

/// A repository as shown in lists and overview pages.
#[derive(Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct RepoResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub owner: String,
    pub name: String,
    pub description: Option<String>,
    pub is_private: bool,
    pub clone_url: String,
    /// The requesting user's role: `owner`, `admin`, `write`, `read` or `none`.
    /// The SPA uses this to show/hide management controls.
    pub viewer_role: String,
    #[ts(type = "number")]
    pub star_count: i64,
    #[ts(type = "number")]
    pub watch_count: i64,
    /// Whether the requesting user has starred/watched this repository.
    pub starred: bool,
    pub watching: bool,
}

/// A branch name and the commit it points at.
#[derive(Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct Branch {
    pub name: String,
    pub sha: String,
}

/// One row in a commit list.
#[derive(Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct Commit {
    pub sha: String,
    pub author: String,
    /// ISO-8601 timestamp, kept as a string so the binding needs no chrono shim.
    pub committed_at: String,
    pub message: String,
}

/// The full message and metadata of a single commit.
#[derive(Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CommitDetail {
    pub sha: String,
    pub author: String,
    pub author_email: String,
    pub committed_at: String,
    pub message: String,
    pub parents: Vec<String>,
}

/// One entry returned by a tree listing.
#[derive(Debug, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct TreeEntry {
    pub name: String,
    pub path: String,
    /// `blob`, `tree` or `commit` (submodule).
    pub kind: String,
    #[ts(type = "number")]
    pub size: i64,
    pub sha: String,
}
