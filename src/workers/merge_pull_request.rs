//! Performs the git side of a pull request merge off the request path.
//!
//! Only identifiers are persisted in the job; the worker reloads current rows
//! so it never acts on a stale snapshot. A conflict returns `Err` (the queue
//! retries it); the PR stays `open` for the author to fix and retry.

use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::data::{git::GitRepo, settings::AppSettings};
use crate::models::{events, organizations, pull_requests, repositories, users};

pub struct Worker {
    pub ctx: AppContext,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct WorkerArgs {
    pub pull_request_id: i64,
    /// Who requested the merge; used to attribute the activity event.
    pub actor_id: i64,
}

#[async_trait]
impl BackgroundWorker<WorkerArgs> for Worker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    fn class_name() -> String {
        "MergePullRequest".to_string()
    }

    fn max_retries() -> u32 {
        3
    }

    async fn perform(&self, args: WorkerArgs) -> Result<()> {
        let pr = pull_requests::Entity::find_by_id(args.pull_request_id)
            .one(&self.ctx.db)
            .await?
            .ok_or_else(|| Error::string("pull request no longer exists"))?;
        // The endpoint only enqueues open PRs, but a merge could be retried.
        if !pr.is_open() {
            return Ok(());
        }

        let repo = repositories::Entity::find_by_id(pr.repository_id)
            .one(&self.ctx.db)
            .await?
            .ok_or_else(|| Error::string("repository no longer exists"))?;
        // An org-owned repo lives under the org's handle, not the creator's.
        let owner_handle = if let Some(org_id) = repo.organization_id {
            organizations::Entity::find_by_id(org_id)
                .one(&self.ctx.db)
                .await?
                .ok_or_else(|| Error::string("repository organization no longer exists"))?
                .handle
        } else {
            users::Entity::find_by_id(repo.user_id)
                .one(&self.ctx.db)
                .await?
                .ok_or_else(|| Error::string("repository owner no longer exists"))?
                .owner_handle()
                .to_owned()
        };

        let settings = AppSettings::from_context(&self.ctx)?;
        let git = GitRepo::open(&settings.repo_base_dir, &owner_handle, repo.display_name()?)?;
        let message = format!(
            "Merge pull request #{} from {} ({})",
            pr.number, pr.head_ref, pr.title
        );
        let commit = git.merge(&pr.base_ref, &pr.head_sha, &message).await?;

        let active = pull_requests::ActiveModel {
            id: Set(pr.id),
            ..Default::default()
        };
        active.mark_merged(&self.ctx.db, &commit).await?;
        events::Entity::record(
            &self.ctx.db,
            repo.id,
            args.actor_id,
            events::kind::PR_MERGED,
            Some(json!({ "number": pr.number })),
        )
        .await?;
        tracing::info!(pull_request_id = pr.id, %commit, "pull request merged");
        Ok(())
    }
}
