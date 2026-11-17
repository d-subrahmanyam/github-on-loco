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

Use your **username** (not your display name) in the URL. The token is the
Basic-auth password.

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

- **Conversation** — general or file/line-anchored review comments.
- **Commits & files** — the comparison: commits ahead/behind the base, and one
  expandable panel per changed file with additions, deletions and the patch.

The diff is a three-dot comparison from the merge base, so it shows what your
branch actually introduces rather than the raw difference between the two tips.

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

- **Only the repository owner can open or merge PRs.** There are no
  collaborators, roles or forks yet ([roadmap](roadmap.md), Phase 4), so both
  branches must live in the same repository.
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
