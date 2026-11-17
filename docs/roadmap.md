# Roadmap

Status legend: ✅ done · 🟡 partial · ⬜ planned.

## Phase 0 — Foundations ✅

- [x] Canonical `username` handle: migration, unique index, backfill, fixtures.
- [x] Registration accepts `username` and derives a safe fallback from `name`.
- [x] Ownership checks in the git transport use `owner_handle()`.
- [x] Repo-name/owner validation and path building centralized on the model.
- [x] Repo creation rolls back the DB row if `git init --bare` fails.
- [x] Personal access tokens for git authentication.
- [x] DTOs with `ts-rs` exports instead of entities on the wire.
- [x] Unit + request tests for the above.

## Phase 1 — Code browsing ✅

- [x] `GitRepo` read layer (branches, commits, commit detail, tree, blob).
- [x] `GET /api/repos/{owner}/{repo}` overview.
- [x] branches / commits / commit / tree / raw endpoints.
- [x] Ref and path safety checks with unit tests.
- [x] SPA pages in `frontend/`: dashboard (list/create), code browser, commit
  history/detail, branch list.
- [x] Redux Toolkit + Redux Saga for client state and API side effects
  (`frontend/src/store/`), with typed hooks and stale-response protection.
- [x] Read-through branch/commit cache in Postgres, invalidated on push; only
  the default branch's history is cached.

## Phase 2 — Issues and labels ⬜

- `cargo loco generate model issues ...`, `issue_comments`, `labels`, `issue_labels`.
- Model state transitions: `open`, `close`, `reopen`; per-repo numbering.
- Endpoints and SPA: issue list, new issue, detail, comments, label filters.
- Markdown rendering in the frontend.

## Phase 3 — Pull requests ⬜

- `pull_requests` model with per-repo numbering and `open|closed|merged` state.
- Compare endpoint using `git diff base...head`.
- Review comments, then a merge endpoint that performs the merge in a worker.
- Branch protection rules after permissions exist.

## Phase 4 — Collaboration and permissions ⬜

- `collaborators`, organizations, teams, roles.
- A single `can(user, repo, action)` model method used by REST *and* git auth.
- Enforce private repositories on browse and clone/push paths.
- Stars, forks, watches, activity events from `events`.

## Phase 5 — Polish and scale ⬜

- Releases/tags, search, notification emails via the mailer + queue, webhooks.
- Move repo storage to a shared volume or object storage for multi-instance.
- Background workers for heavy git operations; scheduler for ref sync.
- Rate limiting and audit logs.

## Phase 6 — Production hardening ⬜

- Move JWT secret and DB/Redis URLs out of checked-in YAML into secrets.
- CI running migrations, `clippy -D warnings` and the full test suite.
- Backup/restore for both Postgres and the git repositories.
- Observability: structured logs, metrics, tracing.

## Ordering rule

Prefer **vertical slices** over finishing a layer: a milestone should be
demoable in the browser. The next slice after this one is *login → create repo
→ push → browse code and issues*.
