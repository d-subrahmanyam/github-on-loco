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
3. The resolved user is checked with `can(db, user_id, action)` against the
   repository at the path. The Basic username is only a hint for the
   password-fallback path; PATs encode their own owner. A user other than the
   owner may push if they have a `write`/`admin` collaborator grant, or a team
   grant on an organization-owned repository.

## Failure responses

| Situation | Status | Body / header |
|---|---|---|
| No credentials, or credentials that do not resolve to a user | `401` | `authentication required`, with `WWW-Authenticate: Basic realm="Git"` so git re-prompts |
| Valid credentials, but no `Write` on a **public** repo | `403` | `permission denied: write access to this repository is required`, no `WWW-Authenticate` |
| Repository does not exist, caller authenticated | `404` | `repository not found` |
| Repository does not exist, caller anonymous | `401` | same as "no credentials", so a missing repo cannot be distinguished from a private one |
| Private repo the caller may not read — anonymously or with insufficient grant | `404` | same as a missing repo — existence is never leaked |

`WWW-Authenticate` is deliberately absent from `403` and `404`: re-prompting
with the same credentials cannot help, and git reports the response body as the
reason instead of looping. Private repositories use `404` for every no-access
case, matching the REST endpoints and the SPA.

## Setting a remote

Use the **git** URL from the repository page (the clone URL in the API), which
is `/<owner>/<repo>.git`. The SPA route `/repos/<owner>/<repo>` serves HTML and
is not a git endpoint.

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
  be a PAT or (locally) the account password. See [Failure responses](#failure-responses)
  for which status code each kind of rejection produces.
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

## Tags and archives

Releases create lightweight tags through `GitRepo::create_tag` (`git update-ref
refs/tags/<tag> <sha> 0000…`). The all-zero old value is a compare-and-swap, so a
tag that already exists is rejected with `400` rather than moved. Deleting a
release removes its tag, and loose tags can be deleted from the Releases tab.
`GET /api/repos/{owner}/{repo}/archive/{ref}.tar.gz` streams `git archive` output
with `application/gzip`; the whole filename is one path segment because Loco's
router does not allow a literal suffix after a path parameter.

## Known gaps

- The clone URL is built from `ctx.config.server.full_url()`; behind a proxy,
  configure `server.host`/port so advertised URLs are correct.
- Repo creation is synchronous. Moving `git init --bare` into a worker is
  worthwhile once repo creation becomes common.
