# Git transport

Git clients do not speak the app's JSON API. Loco's `git_http.rs` implements
the *server* side of git's smart HTTP protocol by delegating to the reference
implementation, `git http-backend`.

## Routes

| Route | Method | Purpose | Auth |
|---|---|---|---|
| `/{owner}/{repo}/info/refs?service=...` | GET | Advertise refs; also starts push when `service=git-receive-pack` | push only |
| `/{owner}/{repo}/git-upload-pack` | POST | fetch/clone objects | anonymous |
| `/{owner}/{repo}/git-receive-pack` | POST | accept pushed objects | required |

The `repo` segment must end in `.git` and its bare name must pass
`repositories::validate_name`.

## How `http-backend` is invoked

`run_git_http_backend` spawns `git http-backend` with a CGI-like environment:

- `GIT_PROJECT_ROOT` — `settings.repo_base_dir`;
- `GIT_HTTP_EXPORT_ALL=1` — allows serving every repo under the root;
- `PATH_INFO` — `/{owner}/{repo}/<git command>`;
- `REQUEST_METHOD`, `QUERY_STRING`, `CONTENT_TYPE`, `CONTENT_LENGTH`;
- `REMOTE_USER` — the authenticated owner, when applicable.

The request body is written to the child's stdin. The child's stdout is split
at the first blank line (`\r\n\r\n` or `\n\n`); the head is parsed into HTTP
headers (including `Status:`) and the remainder becomes the response body.
No shell is involved, so user input cannot inject commands.

## Authentication

Clones and fetches are anonymous today. Pushes require HTTP Basic auth:

1. The Basic **password** is tried as a personal access token:
   `personal_access_tokens::Entity::authenticate` parses `glp_<id>_<secret>`,
   loads the row, verifies the secret against its stored hash, checks expiry,
   and returns the owning user.
2. If that fails, the password is tried against the account password hash. The
   Basic username may be either the email or the `username` handle. This is a
   local-development convenience; a PAT is the supported way to push remotely.
3. In both cases the resolved user's `owner_handle()` must equal the `owner`
   path segment. A valid credential for a different user is rejected with
   `401` plus `WWW-Authenticate: Basic realm="Git"`.

## Setting a remote

```sh
# PAT as the password; the Basic username can be anything
git clone http://localhost:5150/alice/hello.git
cd hello
git remote set-url origin http://alice:<PAT>@localhost:5150/alice/hello.git
git push origin main
```

## Visibility

Private repositories require authentication for **all** access:

- browse REST endpoints call `repositories::Model::can_read(viewer_id)` and
  answer 404 for anonymous or non-owner viewers;
- `git http-backend` routes require Basic auth on both `info/refs` (any
  service) and `git-upload-pack`; pushes were already authenticated;
- issue and label endpoints use the same `can_read` rule, with owner-only
  checks for state changes and label creation.

This is the single authorization rule; collaborators (Phase 4) will extend it
rather than add a second permission system.

## Known gaps

- The clone URL is built from `ctx.config.server.full_url()`; behind a proxy,
  configure `server.host`/port so advertised URLs are correct.
- Repo creation is synchronous. Moving `git init --bare` into a worker is
  worthwhile once repo creation becomes common.
