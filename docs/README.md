<img src="../assets/github-loco-logo.png" alt="github-on-loco logo" width="120" />

# Documentation index

This folder is the durable design record for the project. Chat decisions rot;
these files are the contract.

| Document | What it answers |
|---|---|
| [architecture.md](architecture.md) | How the app is split up, where state lives, how a request flows, and why |
| [deployment-architecture.md](deployment-architecture.md) | Recommended infrastructure today, module boundaries to enforce, and the later microservices/events/service-mesh design |
| [data-model.md](data-model.md) | What is in Postgres now and what the GitHub-shaped model looks like next |
| [design-practices.md](design-practices.md) | The conventions every change must follow, plus a review checklist |
| [git-transport.md](git-transport.md) | How clone/fetch/push work, and how authentication is handled |
| [pull-request-workflow.md](pull-request-workflow.md) | Step-by-step guide for users: token, clone, branch, push, open a PR, review and merge |
| [roadmap.md](roadmap.md) | Phased plan from starter to a useful GitHub-like product, with status |
| [testing.md](testing.md) | Test layers, how to run them, and what is covered today |

## What this app is right now

A Loco 1.3 SaaS starter extended with the first vertical slice of a GitHub-like
product:

- JWT auth, email verification, password reset and magic links (`controllers/auth.rs`);
- `users` with a unique public `username` handle (`models/users.rs`);
- create/list repositories, each with a bare git repo on disk
  (`controllers/repositories.rs`, `models/repositories.rs`). Creation refuses to
  reuse an existing bare repo, and `cargo loco task dev:reset` clears Postgres,
  git storage and the fixture users together;
- clone/fetch/push over git smart HTTP (`controllers/git_http.rs`);
- personal access tokens for git authentication (`models/personal_access_tokens.rs`);
- read-only code browsing — branches, commits, tree, raw files
  (`data/git.rs`, `controllers/git_browse.rs`), with a read-through Postgres cache
  for branch heads and default-branch commits;
- issues and labels with per-repository numbering (`models/issues.rs`,
  `controllers/issues.rs`);
- pull requests with a three-dot compare view, review comments and a background
  merge worker (`models/pull_requests.rs`, `data/git.rs`,
  `workers/merge_pull_request.rs`) — see
  [pull-request-workflow.md](pull-request-workflow.md) for the user guide;
- collaborators with `read`/`write`/`admin` roles and one `can(user, action)`
  rule shared by REST and git auth (`models/collaborators.rs`,
  `models/repositories.rs`, `controllers/git_http.rs`);
- stars, watches and an append-only activity log with an SPA feed
  (`models/stars.rs`, `models/watches.rs`, `models/events.rs`,
  `controllers/social.rs`);
- organizations and teams: org-owned repositories, membership roles, teams and
  per-team repo grants folded into the one `can()` rule
  (`models/organizations.rs`, `models/team_*`, `controllers/organizations.rs`);
- forks: a bare git copy plus a source/fork edge, with fork counts and a fork
  list (`models/forks.rs`, `controllers/forks.rs`);
- branch protection rules and PR approvals: push blocks and required reviewers
  (`models/branch_protection_rules.rs`, `models/pr_reviews.rs`,
  `controllers/protection.rs`, `controllers/git_http.rs`);
- a React SPA (`frontend/`) with login/register, a repository dashboard, code
  browser, commit history/detail, branch list, issue tracker, pull request and
  branch-protection settings pages. Client state and all API side effects are
  handled by **Redux Toolkit +
  Redux Saga** (`frontend/src/store/`).

## Run it

```sh
cargo loco start                 # http://localhost:5150
cargo loco routes                # prove an endpoint is actually wired
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

See [testing.md](testing.md) for the test database and `REPO_BASE_DIR` details.
