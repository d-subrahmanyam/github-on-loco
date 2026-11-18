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

Redis is configured because the dev queue is Redis, and the app boots the queue
provider on test startup. Jobs themselves run inline: `config/test.yaml` uses
`ForegroundBlocking`, so `perform_later` completes before the assertion runs and
the merge worker is exercised synchronously by the PR request tests.

## Layers and where they live

| Layer | Location | Needs external services |
|---|---|---|
| Validation, token format, git path safety, content-type mapping | `#[cfg(test)] mod tests` inside `src/` | no |
| Model queries and transitions | `tests/models/` | Postgres |
| HTTP contract | `tests/requests/` | Postgres, Redis, `git` binary |
| Tasks / workers | `tests/tasks/`; the merge worker is exercised through `tests/requests/pulls.rs` in `ForegroundBlocking` mode | Postgres, `git` |

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
- **Pull requests**: model tests for numbering/state/merge recording, review-comment
  scoping and push-driven head-SHA refresh; request tests exercise the full flow
  against a real bare repo (open → compare → comment → merge → browse the merged
  file), private visibility, auth requirements, a genuine merge conflict that
  leaves the PR open, and merging after a late push/reopen.
  `support::seed_branch_commit` builds divergent branches for these tests.
- **Collaborators and permissions**: unit tests for role → action mapping;
  model tests for grants, role changes, revocation and the unified `can` rule;
  request tests for add/update/remove, invalid roles, non-admin rejection, and
  the same grant taking effect over REST *and* git (read can clone a private
  repo, write can push, removal revokes both).
- **Stars, watches and events**: model tests for idempotent toggles and counts;
  request tests for toggle counts, `viewer_role`-free anonymous behaviour,
  private-repo visibility, and events produced by stars/watches/issues/PR
  merges. A unit test parses the branch out of a receive-pack pkt-line body.
- **Organizations and teams**: model tests for the creator-not-owner rule, org
  roles, team membership and grants, and the resulting `can()`/`role_for()`
  answers; request tests for owner-only settings, creating a repo under an org,
  a member gaining and losing git push access as a team grant is added/revoked,
  and a personal repo with the same name not shadowing an org repo.
- **Webhooks**: model tests for event subscription matching (including the
  wildcard), validation, secret rotation and delivery status transitions; a
  deterministic HMAC test; request tests for admin-only settings, masked
  secrets, duplicate URLs, an end-to-end signed POST to a local HTTP receiver,
  and the recorded delivery history. Frontend tests cover the saga and the
  admin-only Webhooks page.
- **Releases and tags**: model tests for per-repo tag uniqueness, prerelease
  exclusion from `latest`, edits and validation; request tests that publishing
  creates and deleting removes the git tag, that duplicate tags and unsafe names
  are rejected, that `git archive` returns gzip bytes, and that private repos
  and non-writers get 404. Frontend tests cover the release saga and the Releases
  page's tag/archive links.
- **Forks**: model tests for the source/fork edge and label lookup; request tests
  that a fork copies both branches of the bare repo, appears in the source's
  fork count/list and the fork's own overview, avoids name collisions, rejects
  self-forking, and honours sign-in and private-repo visibility.
- **Branch protection and reviews**: model tests for rule upsert/validation and
  review verdict changes (including excluding the author's own approval);
  request tests for admin-only settings, blocked merges until approvals arrive,
  self-review rejection, and a synthetic receive-pack push that is rejected for
  a writer but allowed through for the owner. The approval event shows up in the
  activity feed.
- **Git HTTP authorization**: request tests check that missing credentials get
  a 401 with `WWW-Authenticate`, a missing repo gets a clear 404 only for an
  authenticated caller (401 for anonymous, so existence is not leaked), a
  credentialed user without write access to a public repo gets a 403 with the
  reason, and a private repo unanswered by a grant keeps returning 404. Existing
  collaborator and organization tests assert the new 404 for private repos.
- **Frontend**: Vitest + Testing Library (`pnpm test` in `frontend/`) cover the
  repository/fork/issue/organization/pull/collaborator/protection/social/token sagas against a mocked `fetch`
  (including stale-response protection), the password visibility toggle, token
  storage and logout routing, and rendering of the issue, PR, collaborator,
  fork-list, organization, branch-protection, releases and activity pages.
  The `Markdown` component test checks GFM rendering and that raw HTML and
  `javascript:` links are not emitted. `pnpm build` still type-checks everything.
- **Git storage consistency**: unit tests for bare-repo detection, safe storage
  clearing, and that creation refuses to reuse an existing bare repo (409) while
  leaving its data untouched; a request test simulates a lost DB row and checks
  that the create is refused and its metadata rolled back.
- **Tasks**: user create/delete, `dev:reset` (database + git storage + re-seed).

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
