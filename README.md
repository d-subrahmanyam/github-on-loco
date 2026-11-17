# github-on-loco

A GitHub-like code hosting app built with [Loco](https://loco.rs) — "Rails for
Rust". The goal is a self-hostable GitHub: users and repositories, real
`git clone`/`push`, code browsing, and eventually issues, pull requests and
collaboration.

This repository began as Loco's SaaS starter and now implements the first
end-to-end slice of that product.

## What works today

- **Accounts** — registration, JWT login, email verification, password reset,
  magic links. Each user has a unique `username` handle used in repository URLs.
- **Repositories** — create/list through the API or the SPA; each repo gets a
  bare git repository on disk under `REPO_BASE_DIR`.
- **Git transport** — `git clone`, `fetch` and `push` over HTTP via
  `git http-backend`. Pushes require Basic auth (account password in local
  development; personal access tokens are modelled and supported by the
  transport, but no token-management endpoint exists yet).
- **Code browsing** — branches, commit history, commit detail, directory trees
  and raw files, exposed as a typed JSON API and rendered by the SPA.
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
The development config falls back to a clearly-labelled dev-only value.

## HTTP surface

| Area | Endpoints |
|---|---|
| Auth | `/api/auth/register`, `/login`, `/current`, `/verify`, `/forgot`, `/reset`, `/magic-link` |
| Repositories | `GET/POST /api/repositories` |
| Browse | `/api/repos/{owner}/{repo}` (`/branches`, `/commits`, `/commit/{sha}`, `/tree`, `/raw`) |
| Git | `/{owner}/{repo}/info/refs`, `/git-upload-pack`, `/git-receive-pack` |

Run `cargo loco routes` to see the live, complete route table.

## Pushing code

Create a repository in the SPA, then clone it and push with your account
credentials:

```sh
git clone http://localhost:5150/<username>/<repo>.git
cd <repo>
git remote set-url origin http://<email-or-username>:<password>@localhost:5150/<username>/<repo>.git
git push origin main
```

Use a personal access token instead of the password wherever possible —
putting an account password in a git remote is convenient only for local
development.

## Checks

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
(cd frontend && pnpm build)
```

See [docs/testing.md](docs/testing.md) for the test database and
`REPO_BASE_DIR` behaviour. Note that the test configuration recreates the test
database on every boot.

## Not yet implemented

Private-repository enforcement, issues, pull requests, collaborators and
organisations, stars/forks, notifications, search, and horizontal scaling of
git storage. See [docs/roadmap.md](docs/roadmap.md) for the planned phases and
[docs/git-transport.md](docs/git-transport.md) for the current security caveat
around private repositories.
