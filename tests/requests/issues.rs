//! Request tests for the issues and labels API.

use github_loco::{
    app::App,
    dtos::issues::{CommentResponse, IssueResponse, LabelResponse},
};
use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;

use super::{prepare_data, support::TestRepoRoot};

#[tokio::test]
#[serial]
async fn full_issue_lifecycle() {
    let _root = TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let logged = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&logged.token);
        let owner = logged.user.owner_handle().to_owned();
        let base = format!("/api/repos/{owner}/project");

        let repo_response = request
            .post("/api/repositories")
            .add_header(key.clone(), value.clone())
            .json(&json!({ "name": "project", "is_private": false }))
            .await;
        assert_eq!(repo_response.status_code(), 200, "{}", repo_response.text());

        let label_response = request
            .post(&format!("{base}/labels"))
            .add_header(key.clone(), value.clone())
            .json(&json!({ "name": "bug", "color": "#ff0000" }))
            .await;
        assert_eq!(
            label_response.status_code(),
            200,
            "{}",
            label_response.text()
        );
        let label: LabelResponse = serde_json::from_str(&label_response.text()).unwrap();
        assert_eq!(label.name, "bug");
        assert_eq!(label.color, "ff0000");

        let created = request
            .post(&format!("{base}/issues"))
            .add_header(key.clone(), value.clone())
            .json(&json!({
                "title": "Something broke",
                "body": "details",
                "label_ids": [label.id]
            }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());
        let issue: IssueResponse = serde_json::from_str(&created.text()).unwrap();
        assert_eq!(issue.number, 1);
        assert_eq!(issue.state, "open");
        assert_eq!(issue.author, owner);
        assert_eq!(issue.labels.len(), 1);
        assert_eq!(issue.labels[0].name, "bug");

        let listed = request.get(&format!("{base}/issues")).await;
        assert_eq!(listed.status_code(), 200, "{}", listed.text());
        let issues: Vec<IssueResponse> = serde_json::from_str(&listed.text()).unwrap();
        assert_eq!(issues.len(), 1);

        let shown = request.get(&format!("{base}/issues/1")).await;
        assert_eq!(shown.status_code(), 200, "{}", shown.text());
        let shown: IssueResponse = serde_json::from_str(&shown.text()).unwrap();
        assert_eq!(shown.title, "Something broke");

        let comment = request
            .post(&format!("{base}/issues/1/comments"))
            .add_header(key.clone(), value.clone())
            .json(&json!({ "body": "I can reproduce it" }))
            .await;
        assert_eq!(comment.status_code(), 200, "{}", comment.text());
        let comment: CommentResponse = serde_json::from_str(&comment.text()).unwrap();
        assert_eq!(comment.author, owner);

        let closed = request
            .post(&format!("{base}/issues/1/close"))
            .add_header(key.clone(), value.clone())
            .await;
        assert_eq!(closed.status_code(), 200);
        let closed: IssueResponse = serde_json::from_str(&closed.text()).unwrap();
        assert_eq!(closed.state, "closed");
        assert_eq!(closed.comment_count, 1);

        let reopened = request
            .post(&format!("{base}/issues/1/reopen"))
            .add_header(key, value)
            .await;
        assert_eq!(reopened.status_code(), 200);
        let reopened: IssueResponse = serde_json::from_str(&reopened.text()).unwrap();
        assert_eq!(reopened.state, "open");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn rejects_blank_issue_titles_and_bad_labels() {
    let _root = TestRepoRoot::new();

    request::<App, _, _>(|request, ctx| async move {
        let logged = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&logged.token);
        let owner = logged.user.owner_handle().to_owned();
        let base = format!("/api/repos/{owner}/project");

        request
            .post("/api/repositories")
            .add_header(key.clone(), value.clone())
            .json(&json!({ "name": "project", "is_private": false }))
            .await;

        let blank = request
            .post(&format!("{base}/issues"))
            .add_header(key.clone(), value.clone())
            .json(&json!({ "title": "   " }))
            .await;
        assert_eq!(blank.status_code(), 400);

        let bad_label = request
            .post(&format!("{base}/issues"))
            .add_header(key, value)
            .json(&json!({ "title": "Valid", "label_ids": [999999] }))
            .await;
        assert_eq!(bad_label.status_code(), 400);
    })
    .await;
}
