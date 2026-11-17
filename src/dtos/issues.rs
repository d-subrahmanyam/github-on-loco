//! Wire types for the issues API. Entities never go on the wire directly.

use ts_rs::TS;

/// Body of `POST /api/repos/{owner}/{repo}/issues`.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateIssueParams {
    pub title: String,
    pub body: Option<String>,
    /// Label ids, which must belong to the same repository.
    #[serde(default)]
    pub label_ids: Vec<i64>,
}

/// Body of `POST /api/repos/{owner}/{repo}/issues/{number}/comments`.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct AddCommentParams {
    pub body: String,
}

/// Body of `POST /api/repos/{owner}/{repo}/labels`.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreateLabelParams {
    pub name: String,
    pub color: String,
}

/// A label as shown on an issue.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct LabelResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub color: String,
}

/// One issue in a list or detail response.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct IssueResponse {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub author: String,
    #[ts(type = "number")]
    pub comment_count: i64,
    pub labels: Vec<LabelResponse>,
    pub created_at: String,
    pub updated_at: String,
}

/// A comment on an issue.
#[derive(Debug, serde::Deserialize, serde::Serialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CommentResponse {
    #[ts(type = "number")]
    pub id: i64,
    pub body: String,
    pub author: String,
    pub created_at: String,
}
