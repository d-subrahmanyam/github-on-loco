//! Wire types for the global search API.

use ts_rs::TS;

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct RepoSearchHit {
    #[ts(type = "number")]
    pub id: i64,
    pub owner: String,
    pub name: String,
    pub description: Option<String>,
    pub is_private: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct IssueSearchHit {
    #[ts(type = "number")]
    pub id: i64,
    pub owner: String,
    pub repo: String,
    #[ts(type = "number")]
    pub number: i64,
    pub title: String,
    pub state: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct PullSearchHit {
    #[ts(type = "number")]
    pub id: i64,
    pub owner: String,
    pub repo: String,
    #[ts(type = "number")]
    pub number: i64,
    pub title: String,
    pub state: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct SearchResult {
    pub repositories: Vec<RepoSearchHit>,
    pub issues: Vec<IssueSearchHit>,
    pub pulls: Vec<PullSearchHit>,
    /// Total matches before the per-section limit was applied.
    #[ts(type = "number")]
    pub total: i64,
}
