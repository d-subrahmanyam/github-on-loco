# Git transport

Git clients do not speak the app's JSON API. Loco's `git_http.rs` implements
the *server* side of git's smart HTTP protocol by delegating to the reference
implementation, `git http-backend`.

## Routes

| Route | Method | Purpose | Auth |
|---|---|---|---|
| `/{owner}/{repo}/info/refs?service=...` | GET | Advertise refs; also starts push when `service=git-receive-pack` | `Write` for push, `Read` for private read |
| `/{owner}/{repo}/git-upload-pack` | POST | fetch/clone objects | `Read` for private repos, anonymous for public |
| `/{owner}/{repo}/git-receive-pack` | POST | accept pushed objects | `Write` |

The `repo` segment must end in `.git` and its bare name must pass
`repositories::validate_name`.

## How `http-backend` is invoked

`run_git_http_backend` spawns `git http-backend` with a CGI-like environment:

- `GIT_PROJECT_ROOT` — `settings.repo_base_dir`;
- `GIT_HTTP_EXPORT_ALL=1` — allows serving every repo under the root;
- `PATH_INFO` — `/{owner}/{repo}/<git command>`;
- `REQUEST_METHOD`, `QUERY_STRING`, `CONTENT_TYPE`, `CONTENT_LENGTH`;
- `REMOTE_USER` — the authenticated user's handle (owner or collaborator), when applicable.

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

## Visibility and permissions

Every access — REST or git — goes through one method:
`repositories::Model::can(db, user_id, action)` with actions `Read`, `Write` and
`Admin`. The owner has every action; a public repo grants `Read` to anyone;
other users are checked against their `collaborators` row:
`read` → Read, `write` → Read + Write, `admin` → all three.

- Browse REST endpoints call `can(..., Read)` and answer 404 for viewers without
  a grant — including anonymous viewers of private repos.
- `git http-backend` requires Basic auth for private-repo reads and for every
  push. `info/refs` checks `Read` for `git-upload-pack` and `Write` for
  `git-receive-pack`; `git-upload-pack` and `git-receive-pack` check the same
  actions. The Basic username may be an email or handle, and the password may
  be a PAT or (locally) the account password.
- Issue, label, PR and collaborator endpoints use the same rule: creating or
  changing issues/PRs needs `Write`, managing collaborators needs `Admin`.

Because the git and REST paths share the rule, granting or revoking a
collaborator takes effect immediately on both sides.

## Branch protection

Before `git http-backend` runs, the push handler parses the target branch from
the first receive-pack pkt-line (`refs/heads/<branch>`). If a
`branch_protection_rules` row with `require_pull_request` exists for that branch
and the pusher is **not** an admin (owner or `admin` collaborator), the push is
rejected with `403` and a "merge through a pull request" message. Admins stay
exempt by default, matching GitHub; removing the rule restores direct pushes.

Pull request merges are not blocked by this check: the merge worker updates the
branch with git plumbing (`commit-tree` → guarded `update-ref`) inside the bare
repository, so it never passes through the HTTP push path. A branch with
`required_approvals` also gates merges in the REST endpoint — see
[data-model.md](data-model.md).

## Known gaps

- The clone URL is built from `ctx.config.server.full_url()`; behind a proxy,
  configure `server.host`/port so advertised URLs are correct.
- Repo creation is synchronous. Moving `git init --bare` into a worker is
  worthwhile once repo creation becomes common.
