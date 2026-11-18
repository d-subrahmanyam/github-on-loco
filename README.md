<div align="center">
  <img src="assets/github-loco-logo.png" alt="github-on-loco logo" width="180" />
</div>

# github-on-loco

A GitHub-like code hosting app built with [Loco](https://loco.rs) — "Rails for
Rust". The goal is a self-hostable GitHub: users and repositories, real
`git clone`/`push`, code browsing, issues and labels, and eventually pull
requests and collaboration.

This repository began as Loco's SaaS starter and now implements the first
end-to-end slice of that product.

## What works today

- **Accounts** — registration, JWT login, email verification, password reset,
  magic links. Each user has a unique `username` handle used in repository URLs.
- **Repositories** — create/list through the API or the SPA; each repo gets a
  bare git repository on disk under `REPO_BASE_DIR`.
- **Git transport** — `git clone`, `fetch` and `push` over HTTP via
  `git http-backend`. Pushes require Basic auth; issue and revoke personal
  access tokens from the **Tokens** page in the SPA. Private repositories
  require auth for reads too.
- **Code browsing** — branches, commit history, commit detail, directory trees
  and raw files, exposed as a typed JSON API and rendered by the SPA.
- **Issues and labels** — per-repository issue numbers, open/closed states,
  coloured labels, comments, and an SPA issue tracker.
- **Pull requests** — per-repository numbering, a three-dot compare view
  (commits + per-file patches), review comments, and merges performed by a
  background worker with a git compare-and-swap guard.
- **Collaborators** — grant `read`, `write` or `admin` roles on a repository.
  A single `can(user, action)` rule drives REST endpoints and git clone/push
  auth, so access changes take effect everywhere at once.
- **Stars, watches and activity** — star/watch buttons with live counts, plus a
  repository activity feed fed by an append-only event log.
- **Caching** — branch heads and default-branch commits are cached in Postgres;
  git remains the source of truth and a push invalidates the cache.
- **Frontend** — React + Vite SPA with Redux Toolkit slices and Redux Saga for
  all API side effects. TypeScript bindings are generated from the Rust DTOs
  with `ts-rs`.

## Architecture in one paragraph

Git owns repository *contents*; Postgres owns *metadata and permissions*. The
controllers stay thin: they parse input, call a model method and render a DTO.
Filesystem and git access is isolated in `src/data/git.rs`, and all naming/path
validation is centralised on the repository model. Detailed design notes live
in [`docs/`](docs/README.md):

| Document | Contents |
|---|---|
| [architecture.md](docs/architecture.md) | Layering, source-of-truth rule, request flows, decisions |
| [data-model.md](docs/data-model.md) | Current tables and the target GitHub schema |
| [design-practices.md](docs/design-practices.md) | Conventions and review checklist |
| [git-transport.md](docs/git-transport.md) | smart-HTTP details and authentication |
| [pull-request-workflow.md](docs/pull-request-workflow.md) | User guide: clone, branch, push, open a PR, review and merge |
| [roadmap.md](docs/roadmap.md) | Phases and what's left |
| [testing.md](docs/testing.md) | Test layers and how to run them |

## Requirements

- Rust toolchain (see `Cargo.toml`; Loco 1.3, SeaORM 2.0)
- PostgreSQL (databases `github_loco_development` and `github_loco_test`, user
  `loco` / `loco` by default)
- Redis (queue provider, `redis://127.0.0.1` by default)
- `git` on `PATH`
- Node.js + pnpm for the frontend

Configuration is in `config/<environment>.yaml`. Values can be overridden with
`DATABASE_URL`, `REDIS_URL`, `JWT_SECRET`, `PORT` and `REPO_BASE_DIR`.

## Quick start

```sh
# 1. Backend
cargo loco db migrate
cargo loco start                 # http://localhost:5150

# 2. Frontend (separate shell, with Vite proxying /api to the backend)
cd frontend
pnpm install
pnpm dev                         # http://localhost:5173
```

The production-style build serves the SPA from Loco: run `pnpm build` in
`frontend/` and `cargo loco start`; `frontend/dist` is served with an SPA
fallback (see the static middleware in `config/development.yaml`).

A production JWT secret has no default — set `JWT_SECRET` before deploying.
Loco decodes the secret as **base64** to derive its HMAC key, so generate one
with `openssl rand -base64 32`; a non-base64 value makes login fail with 401.
The development config uses a clearly-labelled dev-only base64 value.

## HTTP surface

| Area | Endpoints |
|---|---|
| Auth | `/api/auth/register`, `/login`, `/current`, `/verify`, `/forgot`, `/reset`, `/magic-link` |
| Repositories | `GET/POST /api/repositories` |
| Browse | `/api/repos/{owner}/{repo}` (`/branches`, `/commits`, `/commit/{sha}`, `/tree`, `/raw`) |
| Git | `/{owner}/{repo}/info/refs`, `/git-upload-pack`, `/git-receive-pack` |

Run `cargo loco routes` to see the live, complete route table.

## Pushing code

Create a repository in the SPA, create a token on the **Tokens** page, then
clone and push with the token as the Basic password:

```sh
git clone http://localhost:5150/<username>/<repo>.git
cd <repo>
git remote set-url origin http://<username>:<token>@localhost:5150/<username>/<repo>.git
git push origin main
```

For private repositories, the same credentials are required to clone or fetch.
The account password still works as a fallback for local development, but a
token is the right choice for a remote.

## Checks

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
(cd frontend && pnpm build)
(cd frontend && pnpm test)
```

See [docs/testing.md](docs/testing.md) for the test database and
`REPO_BASE_DIR` behaviour. Note that the test configuration recreates the test
database on every boot.

## Not yet implemented

Organisations and teams, forks, branch protection, notifications, search, and
horizontal scaling of git storage. See [docs/roadmap.md](docs/roadmap.md) for
the planned phases.
