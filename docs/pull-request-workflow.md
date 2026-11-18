# Pull request workflow: from clone to merge

A practical guide to contributing a change through the web UI. It assumes the
app is running at `http://localhost:5150` (see the [README](../README.md) for
setup) and that you have an account and a repository to work on.

The short version: **clone → branch → commit → push → open a PR in the SPA →
review → merge**. Git is the source of truth for code; Postgres stores the PR,
its comments and its state.

## Before you start

You need a **personal access token** for any operation that writes to git.
Account passwords still work as a local-development fallback, but a token is
the right choice for a git remote.

Open the SPA, log in, click **Tokens** in the header, and create a token. Copy
the `glp_…` value immediately — the plaintext is shown only once, and only its
hash is stored afterwards.

If you're using a private repository, the token is needed to *clone* as well;
public repositories can be cloned anonymously.

## Who opens the pull request

Both the repository owner and other contributors open pull requests the same
way — clone, branch, commit, push, open the PR. What differs is how they get
permission to push the head branch:

| | Repository owner | Contributor |
|---|---|---|
| Push access | Implicit — the owner has every action | Needs a `write`/`admin` collaborator grant, or a team grant on an org-owned repo |
| Head branch | Pushed to their own repository | Pushed to the same repository (cross-repository PRs are not implemented yet); a fork is a way to work in your own namespace, but its branch cannot be the head of a PR in the source repo today |
| Protected base branch | `require_pull_request` push blocks exempt admins, so the owner can still push directly | Direct pushes to the protected branch are rejected with `403`; use a PR instead |
| Approval | The author cannot approve their own PR, so `required_approvals` still needs another reviewer | Same rule |

The push URL must be the repository's **git** path, `/<owner>/<repo>.git`, where
`<owner>` is the account (or organization) that owns the repository — not
necessarily you. The SPA page at `/repos/<owner>/<repo>` serves HTML and is not
a git endpoint. Set the Basic username to your handle with your PAT as the
password:

```sh
git remote set-url origin http://<your-handle>:<token>@localhost:5150/<repo-owner>/<repo>.git
git push -u origin feature/my-change
```

If the push is rejected, the message tells you which rule stopped it:

- `401 authentication required` — no credentials, or credentials that did not
  resolve to a user;
- `403 permission denied: write access to this repository is required` — your
  credentials are valid but your account has no `Write` grant on that repo; ask
  the owner for a collaborator grant;
- `404 repository not found` — either the `owner` path segment is wrong (a repo
  lives under its owner's handle, which is *your* handle only for your own
  repos), or the repository is private and you have no grant.

Private repositories answer `404` for every no-access case so their existence
is not leaked. The full decision table is in
[git-transport.md](git-transport.md#failure-responses).

## 1. Clone the repository

```sh
# Public repository — no credentials needed to clone
git clone http://localhost:5150/<owner>/<repo>.git

# Private repository — credentials required up front
git clone http://<username>:<token>@localhost:5150/<owner>/<repo>.git
cd <repo>
```

If you cloned a public repository, attach the token to the remote before
pushing:

```sh
git remote set-url origin http://<username>:<token>@localhost:5150/<owner>/<repo>.git
```

Use your **username** (not your display name) in the URL, and use the
repository's owner in the path. The token is the Basic-auth password. See
[Who opens the pull request](#who-opens-the-pull-request) for the owner vs.
contributor permission rules and what each push failure means.

## 2. Make your change on a branch

```sh
git checkout -b feature/my-change

# edit files, then:
git add .
git commit -m "Describe the change"
git push -u origin feature/my-change
```

The head branch must exist on the remote before you can open a PR — the SPA
only lists branches it can see in the bare repository. A successful push clears
the branch cache, so reload the Pull requests page if you already had it open
and your new branch isn't in the dropdown.

## 3. Open the pull request

In the SPA, open the repository and select the **Pull requests** tab, then
**New pull request**:

| Field | What to put there |
|---|---|
| Base branch | The branch you want to change, usually `main` |
| Compare branch | The branch you just pushed, e.g. `feature/my-change` |
| Title / description | What the change does and why |

Submit the form and you'll land on the PR detail page.

## 4. Review and merge

The detail page has two tabs:

- **Conversation** — reviewers' verdicts (approve / request changes / comment
  only) followed by general or file/line-anchored review comments.
- **Commits & files** — the comparison: commits ahead/behind the base, and one
  expandable panel per changed file with additions, deletions and the patch.

The diff is a three-dot comparison from the merge base, so it shows what your
branch actually introduces rather than the raw difference between the two tips.

If the base branch is protected, the page shows the required approval count and
the merge button stays disabled until enough reviewers have approved. An admin
can change these rules on the repository's **Branch protection** tab; you cannot
approve your own PR.

When you're happy, use **Merge pull request**. The endpoint hands the work to a
background worker; on success the PR becomes `merged`, the resulting commit SHA
is recorded, and the base branch advances. The merged file is immediately
visible in the code browser.

The merge uses git plumbing inside the bare repository (`merge-tree` →
`commit-tree` → a guarded `update-ref`). There is no working copy involved, and
the compare-and-swap guard prevents a concurrent push from being overwritten.

**Close** and **Reopen** are also available while a PR is open. A merged PR is
final — it cannot be reopened or closed afterwards.

## Limitations to be aware of

- **You need write access to open or merge PRs.** The owner has it implicitly;
  `write`/`admin` collaborators and members of a team with a `write`/`admin`
  grant on an org-owned repo have it too. `read` collaborators can browse and
  comment but not create or merge. An admin can grant access on the repository's
  **Collaborators** tab (a `write` grant is enough). Both branches must still
  live in the same repository: you can fork a repository, but cross-repository
  pull requests are not implemented yet — see
  [Who opens the pull request](#who-opens-the-pull-request).
- **The head follows the branch.** The recorded head SHA is refreshed after
  every push, so pushing another commit (or force-pushing) to your branch
  updates the existing PR's compare view and merge target. Reload the PR page
  to see the new commit.
- **Merge conflicts leave the PR open.** The worker reports the conflict and no
  refs are changed. To recover, merge the base branch into your head branch,
  resolve the conflicts locally, and push again — the PR picks up the new head
  and you can retry the merge.
- In a production deployment the queue runs in `BackgroundQueue` mode, so a
  merge is asynchronous: the PR stays `open` until the worker finishes. The SPA
  refreshes the PR after enqueueing; reload the page to see the final state.
