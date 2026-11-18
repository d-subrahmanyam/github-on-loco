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

## Phase 2 — Issues and labels ✅

- [x] Generated `issues`, `issue_comments`, `labels` and `issue_labels` models/migrations.
- [x] Per-repository issue numbering allocated inside a transaction; labels
      scoped to a repository and validated when attached.
- [x] `open`/`closed` state transitions, comments, and label CRUD endpoints.
- [x] SPA: issue list with state filter, new-issue form, issue detail with
      close/reopen and commenting; labels shown as coloured badges.
- [ ] Markdown rendering in the frontend (plain text bodies for now).

## Phase 3 — Pull requests ✅

- [x] Generated `pull_requests` and `review_comments` models/migrations;
      per-repo numbering allocated in a transaction and `unique(repository_id, number)`.
- [x] `open|closed|merged` states; a merged PR cannot be reopened.
- [x] Compare endpoint using a three-dot diff from the merge base: commits,
      ahead/behind counts and per-file status/counts/patches.
- [x] Review comments (general or anchored to a file and line).
- [x] Merge endpoint that enqueues a background worker. The merge uses git
      plumbing inside the bare repo (`merge-tree` → `commit-tree` → guarded
      `update-ref`), so no working copy is needed and a concurrent push cannot
      be overwritten. Conflicts leave the PR open.
- [x] SPA pull request list, new-PR form, detail with compare/diff and comments,
      close/reopen and merge controls.
- Branch protection rules were implemented in Phase 4 and gate PR merges here.

## Phase 4 — Collaboration and permissions 🟡

- [x] Owner-only private repo enforcement on browse, git transport and issues
      (pulled forward because it was a security gap); token issue/revoke API
      and SPA page.
- [x] `collaborators` table with `read`/`write`/`admin` roles, per-repository
      unique grants, and a manager in the SPA.
- [x] One `repositories::Model::can(user, action)` rule used by **all** REST
      handlers and by git HTTP auth. `can_read`/`can_write`/`can_admin` are thin
      wrappers, so there is no second permission system.
- [x] Git clone/fetch/push follow the same rule: a `read` collaborator can clone
      a private repo, `write` can also push, `admin` can manage access.
- [x] `viewer_role` on the repository overview lets the SPA show only the
      controls the current user is allowed to use.
- [x] Stars and watches: toggle endpoints, counts and the viewer's state on the
      repository overview, and buttons in the repo header.
- [x] Append-only `events` log with an SPA activity feed. Issue/PR actions,
      collaborator changes, star/watch toggles and pushes all record events.
- [x] Branch protection rules per branch: block direct pushes (admins exempt)
      and require a configurable number of non-author approvals before merge.
      Reviews are upserted per reviewer; the PR DTO carries merge readiness so
      the SPA and the authoritative merge check agree.
- [x] Forks: a `forks` edge plus a bare `git clone --bare`, fork/list endpoints,
      fork counts and "forked from" metadata on the repo overview, and an SPA
      fork button and fork list. Fork names avoid collisions with a `-fork-N`
      suffix.
- [x] Organizations and teams: org-owned repositories (a second owner kind),
      org membership roles, teams with members, and per-team repository grants.
      The single `can()` rule now derives repo access from org ownership plus
      team grants, so REST and git auth both follow the same policy.

## Phase 4 status: complete ✅

## Phase 5 — Polish and scale ⬜

- [x] Releases and tags: release metadata in Postgres, compare-and-swap git tags,
      `latest` release lookup, loose-tag management, source archives and an SPA
      Releases tab.
- Remaining: search, notification emails via the mailer + queue, webhooks.
- Deployment and scaling design is recorded in
  [docs/deployment-architecture.md](deployment-architecture.md): shared git
  storage and split web/worker deploys first, then a modular-monolith boundary
  pass, then a strangler migration to per-context services with an event broker
  and a service mesh.
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
