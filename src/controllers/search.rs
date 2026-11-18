//! Global search over repositories, issues and pull requests.
//!
//! Results are filtered through the same `can(user, Read)` rule as every other
//! endpoint, so a private repository cannot be discovered by searching its
//! name or the title of one of its issues.

#![allow(clippy::unused_async)]

use loco_rs::prelude::*;
use sea_orm::EntityTrait;
use serde::Deserialize;

use crate::dtos::search::{IssueSearchHit, PullSearchHit, RepoSearchHit, SearchResult};
use crate::models::{issues, organizations, pull_requests, repositories, users};

const MIN_QUERY_LEN: usize = 2;
const SECTION_LIMIT: u64 = 20;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

fn matches(term: &str, title: &str, body: Option<&str>) -> bool {
    let needle = term.to_lowercase();
    title.to_lowercase().contains(&needle)
        || body.is_some_and(|body| body.to_lowercase().contains(&needle))
}

/// Resolves the owner handle for a repository row (user or organization).
async fn owner_of(db: &sea_orm::DatabaseConnection, repo: &repositories::Model) -> Result<String> {
    if let Some(organization_id) = repo.organization_id {
        Ok(organizations::Entity::find_by_id(organization_id)
            .one(db)
            .await?
            .map(|org| org.handle)
            .unwrap_or_else(|| "unknown".to_owned()))
    } else {
        Ok(users::Entity::find_by_id(repo.user_id)
            .one(db)
            .await?
            .map(|user| user.username)
            .unwrap_or_else(|| "unknown".to_owned()))
    }
}

/// `GET /api/search?q=...`.
#[debug_handler]
pub async fn search(
    State(ctx): State<AppContext>,
    Query(query): Query<SearchQuery>,
    viewer: Option<auth::JWT>,
) -> Result<Response> {
    let term = query.q.trim();
    if term.len() < MIN_QUERY_LEN || term.len() > 100 {
        return bad_request("search query must be between 2 and 100 characters");
    }
    let viewer_id = match viewer {
        Some(jwt) => Some(
            users::Model::find_by_pid(&ctx.db, &jwt.claims.pid)
                .await?
                .id,
        ),
        None => None,
    };
    // Resolve readability once per repository. Issue and PR hits are then
    // limited to that set, so no private metadata leaks through a search.
    let all_repos = repositories::Entity::find().all(&ctx.db).await?;
    let mut readable = Vec::new();
    for repo in all_repos {
        if repo.can_read(&ctx.db, viewer_id).await? {
            readable.push(repo);
        }
    }

    let mut repo_hits = Vec::new();
    for repo in &readable {
        let is_match = matches(
            term,
            repo.name.as_deref().unwrap_or(""),
            repo.description.as_deref(),
        );
        if !is_match {
            continue;
        }
        let owner = owner_of(&ctx.db, repo).await?;
        repo_hits.push(RepoSearchHit {
            id: repo.id,
            owner,
            name: repo.display_name()?.to_owned(),
            description: repo.description.clone(),
            is_private: repo.is_private(),
        });
    }
    let repo_total = repo_hits.len() as i64;
    repo_hits.truncate(SECTION_LIMIT as usize);

    let readable_ids: Vec<i64> = readable.iter().map(|repo| repo.id).collect();
    let paths: std::collections::HashMap<i64, (String, String)> = {
        let mut map = std::collections::HashMap::new();
        for repo in &readable {
            map.insert(
                repo.id,
                (
                    owner_of(&ctx.db, repo).await?,
                    repo.display_name()?.to_owned(),
                ),
            );
        }
        map
    };

    let issue_rows = issues::Entity::find()
        .filter(issues::Column::RepositoryId.is_in(readable_ids.clone()))
        .order_by_desc(issues::Column::Id)
        .limit(500)
        .all(&ctx.db)
        .await?;
    let mut issue_hits = Vec::with_capacity(SECTION_LIMIT as usize);
    for issue in issue_rows
        .into_iter()
        .filter(|row| matches(term, &row.title, row.body.as_deref()))
    {
        if let Some((owner, repo)) = paths.get(&issue.repository_id) {
            issue_hits.push(IssueSearchHit {
                id: issue.id,
                owner: owner.clone(),
                repo: repo.clone(),
                number: issue.number,
                title: issue.title,
                state: issue.state,
            });
        }
        if issue_hits.len() >= SECTION_LIMIT as usize {
            break;
        }
    }

    let pull_rows = pull_requests::Entity::find()
        .filter(pull_requests::Column::RepositoryId.is_in(readable_ids))
        .order_by_desc(pull_requests::Column::Id)
        .limit(500)
        .all(&ctx.db)
        .await?;
    let mut pull_hits = Vec::with_capacity(SECTION_LIMIT as usize);
    for pull in pull_rows
        .into_iter()
        .filter(|row| matches(term, &row.title, row.body.as_deref()))
    {
        if let Some((owner, repo)) = paths.get(&pull.repository_id) {
            pull_hits.push(PullSearchHit {
                id: pull.id,
                owner: owner.clone(),
                repo: repo.clone(),
                number: pull.number,
                title: pull.title,
                state: pull.state,
            });
        }
        if pull_hits.len() >= SECTION_LIMIT as usize {
            break;
        }
    }

    let total = repo_total + issue_hits.len() as i64 + pull_hits.len() as i64;
    format::json(SearchResult {
        repositories: repo_hits,
        issues: issue_hits,
        pulls: pull_hits,
        total,
    })
}

/// Routes registered by `App::routes`.
pub fn routes() -> Routes {
    Routes::new().add("/api/search", get(search))
}
