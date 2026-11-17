# Documentation index

This folder is the durable design record for the project. Chat decisions rot;
these files are the contract.

| Document | What it answers |
|---|---|
| [architecture.md](architecture.md) | How the app is split up, where state lives, how a request flows, and why |
| [data-model.md](data-model.md) | What is in Postgres now and what the GitHub-shaped model looks like next |
| [design-practices.md](design-practices.md) | The conventions every change must follow, plus a review checklist |
| [git-transport.md](git-transport.md) | How clone/fetch/push work, and how authentication is handled |
| [roadmap.md](roadmap.md) | Phased plan from starter to a useful GitHub-like product, with status |
| [testing.md](testing.md) | Test layers, how to run them, and what is covered today |

## What this app is right now

A Loco 1.3 SaaS starter extended with the first vertical slice of a GitHub-like
product:

- JWT auth, email verification, password reset and magic links (`controllers/auth.rs`);
- `users` with a unique public `username` handle (`models/users.rs`);
- create/list repositories, each with a bare git repo on disk
  (`controllers/repositories.rs`, `models/repositories.rs`);
- clone/fetch/push over git smart HTTP (`controllers/git_http.rs`);
- personal access tokens for git authentication (`models/personal_access_tokens.rs`);
- read-only code browsing — branches, commits, tree, raw files
  (`data/git.rs`, `controllers/git_browse.rs`), with a read-through Postgres cache
  for branch heads and default-branch commits;
- a React SPA (`frontend/`) with login/register, a repository dashboard, code
  browser, commit history/detail and branch list. Client state and all API side
  effects are handled by **Redux Toolkit + Redux Saga** (`frontend/src/store/`).

## Run it

```sh
cargo loco start                 # http://localhost:5150
cargo loco routes                # prove an endpoint is actually wired
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

See [testing.md](testing.md) for the test database and `REPO_BASE_DIR` details.
