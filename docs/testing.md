# Testing

## Commands

```sh
cargo test                       # everything
cargo test --lib                 # unit tests next to the code (no DB)
cargo test --test mod models     # model tests (needs Postgres)
cargo test --test mod requests   # HTTP request tests (needs Postgres, Redis, git)
```

`config/test.yaml` uses `dangerously_recreate: true` and `dangerously_truncate:
true`, so **each boot rebuilds the test database**. Set up all data inside the
`request::<App, _, _>(|request, ctx| ...)` closure; data created in a separate
boot will be gone.

Redis is configured because the dev queue is Redis. Tests do not enqueue work
at present, but the app boots the queue provider, so Redis must be reachable
(or use the test config's settings).

## Layers and where they live

| Layer | Location | Needs external services |
|---|---|---|
| Validation, token format, git path safety, content-type mapping | `#[cfg(test)] mod tests` inside `src/` | no |
| Model queries and transitions | `tests/models/` | Postgres |
| HTTP contract | `tests/requests/` | Postgres, Redis, `git` binary |
| Tasks / workers | `tests/tasks/`, `tests/workers/` | Postgres |

`tests/requests/support.rs` provides `TestRepoRoot`, which points
`REPO_BASE_DIR` at a temporary directory *before* the app boots and removes it
on drop. Because it changes process-wide environment, every test using it is
`#[serial]`.

`tests/requests/prepare_data.rs` provides `init_user_login` and `auth_header`
for obtaining a JWT and building the `Authorization` header.

## Coverage today

- **Unit**: repository-name/owner validation, path traversal, git ref/path
  safety, PAT token format and expiry, response content types.
- **Models**: user creation (including derived/duplicate usernames), finders,
  PAT issue/authenticate/reject-expired, entity-vs-migration agreement for all
  three tables.
- **Requests**: registration/login/auth flows (starter), repository
  create/list and unsafe-name rejection, full browse flow (branches → commits
  → commit detail → tree → raw blob) with a real commit pushed into a bare
  repo, cache warming assertions, and rejection of unsafe refs/paths.
- **Cache**: replace/find/clear for both branch and commit caches.
- **Issues**: per-repo numbering, label attachment/validation, state
  transitions, comments and label CRUD; request tests cover the full issue
  lifecycle including authentication failures for blank titles/bad labels.
- **Frontend**: Vitest + Testing Library (`pnpm test` in `frontend/`) cover the
  repository/issue/token sagas against a mocked `fetch` (including stale-response
  protection), the password visibility toggle, token storage and logout routing,
  and rendering of the issue list. `pnpm build` still type-checks everything.
- **Tasks**: user create/delete.

## Writing a new request test

1. Start `TestRepoRoot::new()` if the test touches git storage.
2. Call `request::<App, _, _>(...)`, then log in with `prepare_data`.
3. Exercise the endpoint and assert on `status_code()` and `text()` (or parse
   into the DTO from `src/dtos/`).
4. Keep snapshots narrow — snapshot `(email, name)`, never a whole `Model`, so
   adding a column does not require re-blessing unrelated tests.

## Before you finish

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

A change without a test that fails when the change is reverted is not finished.
