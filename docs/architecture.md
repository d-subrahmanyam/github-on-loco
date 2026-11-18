# Architecture

For deployment topology, the module boundaries needed before a service split,
and the later microservices/events/service-mesh design, see
[deployment-architecture.md](deployment-architecture.md). This document stays
focused on the code's internal structure.

## The central decision: two sources of truth

The easiest way to get a git-hosting app wrong is to make the database and the
git repository compete. They do not compete here; they have different jobs.

| Concern | Authority | Why |
|---|---|---|
| Commits, branches, trees, file contents, diffs | **the bare git repository** | Git already solves history, merging and object integrity. Copying it into SQL creates a cache that can drift from reality. |
| Ownership, visibility, issues, PRs, stars, permissions | **Postgres** | Git has no concept of a user, an issue or an access grant. |

Consequences:

- Browsing endpoints call [`GitRepo`](../src/data/git.rs), never a `commits` SQL
  table as the authority. The `branches` and `commits` tables are a narrow
  **read-through cache**: a miss spawns git and refills the table, and a
  successful push invalidates both caches (`git_http::invalidate_repo_cache`).
  A cache error is swallowed so it can never turn a browse into a 500.
- Only the default branch's commit history is cached; an explicit `?ref=`
  always reads git, so a stale row can never show another branch's history.
- A repository always has **two** artifacts: a row in `repositories` and a
  directory `<repo_base_dir>/<owner>/<name>.git`. Creation is written to roll
  the row back if `git init --bare` fails, so they cannot silently diverge.
- Deleting a repository must eventually delete both; that is a job for a
  background worker, not two unrelated calls in a handler.

## Layers

```
HTTP request
  │
  ├─ middleware (config/*.yaml): logging, compression, static SPA fallback
  │
  ▼
controllers/            parse params, authenticate, call ONE model operation, render a DTO
  │
  ├─ models/            finders, validation, state transitions, domain invariants
  │                     (SeaORM; `_entities/` is generated, never edited)
  │
  ├─ data/git.rs        filesystem/git CLI access — the only place that shells out to git for reads
  │
  └─ data/settings.rs   typed config via ctx.config
```

### Frontend state

The SPA keeps repository and browse state in Redux Toolkit slices
(`frontend/src/store/`). **Redux Saga owns every API side effect**: components
dispatch a `*Requested` action, a saga performs the fetch (`call`) and dispatches
the success/failure action. Browse requests carry a `key` so a late response
from an old branch/path is ignored. The typed `useAppDispatch` / `useAppSelector`
hooks keep components from touching the raw store types.

`AppContext` is the bag of infrastructure every handler receives. The fields
used here are `db`, `config`, `queue_provider`, `mailer`, `cache` and
`shared_store`. Application code reads settings **only** through
`ctx.config` / `AppSettings::from_context`; it never calls `std::env::var`.

## Request flows

### Create a repository

```
POST /api/repositories
  → auth::JWT extractor resolves the JWT to a user
  → users::Model::find_by_pid
  → repositories::ActiveModel::create_for_user  (validates the name, inserts the row)
  → repositories::repo_dir                      (validates owner + name, builds the path)
  → git init --bare <path>
      on failure: delete the row, return the error
  → RepoResponse DTO
```

### Clone / fetch / push

Git has its own protocol; Loco does not reimplement it. `git_http.rs` converts
the incoming request into the CGI environment that `git http-backend` expects
and streams its response back. Details in [git-transport.md](git-transport.md).

### Browse code

```
GET /api/repos/{owner}/{repo}/tree?ref=main&path=src
  → find the metadata row (owner/name → user → repository)
  → GitRepo::open validates the path and checks HEAD exists
  → reference defaults to `git symbolic-ref --short HEAD`
  → git ls-tree <ref>:<path>
  → Vec<TreeEntry> DTO
```

Every user-supplied ref and path passes through `is_safe_ref` / `is_safe_path`
before it reaches a git command line. See the unit tests in `data/git.rs`.

## Naming and path safety

`username` is the single public handle. It appears in clone URLs, is unique,
and is what git authentication compares with the URL owner. `users.name` stays
as a free-form display name.

`repositories::{validate_name, validate_owner, repo_dir}` are the only way user
input becomes a filesystem path. Both validators reject path separators,
`..`, leading/trailing punctuation and over-long values; `repo_dir` then joins
the validated pieces under `settings.repo_base_dir`.

## Storage choice and its limits

Bare repositories live on a local filesystem (`repo_base_dir`, default
`/home/subbu/var/repos`). This is fine for one instance. It does **not** work
for horizontally scaled app servers without a shared volume or object
storage — that is a Phase 5/6 concern, documented in [roadmap.md](roadmap.md).

## Decisions log

| Decision | Choice | Reason |
|---|---|---|
| Git access for browsing | `git` CLI behind `GitRepo` | Matches the existing `http-backend` approach, no FFI dependency, swappable later |
| Git write transport | `git http-backend` CGI | The protocol is complex; use the reference implementation |
| Git auth | PAT (with password fallback in dev), authorized via `can` | JWT cannot travel in a git remote; collaborators must be able to push too |
| Public handle | new `username` column | `name` is display text and was not unique |
| Repo create atomicity | roll back the DB row on git failure | Keeps metadata and disk in step without a distributed transaction |
| Authorization | `repositories::Model::can(user, action)` | One rule for REST, git transport and every feature; wrappers only add names for common actions |
| Activity | append-only `events` log with small display payloads | Git and the domain tables stay authoritative; the feed can be stale without being wrong |
| Forks | a fork is a normal repository plus one edge row | No second repo abstraction: the same permissions, git transport and browsing code work on a fork. Cloning happens once at creation; the edge is metadata only |
| Branch protection | rules live on the branch, enforcement lives at the two write boundaries | Pushes are rejected in `git_http` before `http-backend` runs; merges are checked in the endpoint. Admins stay exempt from push blocks, and PR merges write with git plumbing, so a protected branch is never unreachable |
| Private repo browsing | enforced via `can(..., Read)` | 404 for viewers without a grant avoids leaking existence |
| PR head identity | `head_ref` + tracked `head_sha` | Git stays authoritative; the push handler re-resolves the ref so compare and merge follow new commits |
| PR merge | git plumbing in a background worker (`merge-tree`/`commit-tree`/guarded `update-ref`) | No worktree or extra dependency; compare-and-swap prevents overwriting a concurrent push |
| PR compare | `git diff` from the merge base, not a raw two-dot diff | Three-dot semantics match what a reviewer expects to see |
| Browse cache | read-through, invalidated on push | Avoids spawning git for every list request without letting SQL become authoritative |
| Frontend state | Redux Toolkit + Redux Saga | Explicit, testable side-effect flow for API calls; replaces React Query for repository data |
