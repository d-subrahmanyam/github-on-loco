# Data model

## Implemented today

### `users`

| Column | Notes |
|---|---|
| `id`, `created_at`, `updated_at` | standard Loco fields |
| `pid` (uuid) | public id used in JWT claims |
| `email` (unique), `password` (hash), `api_key` (unique) | auth fields from the starter |
| `name` | display name, not necessarily unique |
| `username` (unique, not null) | **canonical public handle** used in URLs and ownership checks |
| verification / reset / magic-link columns | starter auth flows |

Registration accepts an optional `username`; when omitted,
`users::username_from_name` derives a URL-safe handle from `name`. The
uniqueness check and the insert share a transaction.

### `repositories`

| Column | Notes |
|---|---|
| `id`, `created_at`, `updated_at` | standard |
| `user_id` → `users.id` | owner (cascade delete) |
| `name` | bare name, without `.git`; validated by `repositories::validate_name` |
| `description` | optional |
| `is_private` | missing values are treated as private (fail closed) |

A repository's *contents* are not in this table — they are in the bare git
repository at `<repo_base_dir>/<owner>/<name>.git`.

### `personal_access_tokens`

| Column | Notes |
|---|---|
| `id`, `created_at`, `updated_at` | standard |
| `user_id` → `users.id` | owner (cascade delete) |
| `name` | user-facing label, e.g. "laptop" |
| `token_hash` (unique) | password hash of the token secret; the plaintext is shown once |
| `scopes`, `expires_at`, `last_used_at` | optional metadata |

Token format is `glp_<id>_<secret>`. The id makes lookup possible despite salted
hashes; verification compares the secret against the stored hash.

### `branches` and `commits` (read-through cache)

| Table | Columns |
|---|---|
| `branches` | `repository_id`, `name`, `sha` (+ timestamps) |
| `commits` | `repository_id`, `sha`, `message`, `author`, `committed_at`, `parents` (jsonb, nullable) |

Git is still authoritative. These rows are refilled on a cache miss and cleared
on a successful push. `commits` holds only the default branch's recent history;
explicit `?ref=` requests bypass it.

### `issues`, `issue_comments`, `labels`, `issue_labels` (Phase 2)

| Table | Columns |
|---|---|
| `issues` | `repository_id`, `user_id` (author), `number`, `title`, `body`, `state` (`open`/`closed`) — `unique(repository_id, number)` |
| `issue_comments` | `issue_id`, `user_id`, `body` |
| `labels` | `repository_id`, `name`, `color` (6-digit hex) |
| `issue_labels` | `issue_id`, `label_id` (join table) |

Issue numbers are allocated as `MAX(number) + 1` inside the same transaction as
the insert. Labels are validated to belong to the issue's repository before
being attached.

### `pull_requests`, `review_comments` (Phase 3)

| Table | Columns |
|---|---|
| `pull_requests` | `repository_id`, `user_id` (author), `number`, `title`, `body`, `state` (`open`/`closed`/`merged`), `head_ref`, `head_sha`, `base_ref`, `merged_commit_sha`, `merged_at` — `unique(repository_id, number)` |
| `review_comments` | `pull_request_id`, `user_id`, `body`, `path` (nullable), `line` (nullable) |

The head is stored as both a ref and an immutable SHA resolved when the PR is
opened: compare and merge both use the SHA, so a later force-push cannot change
what was reviewed. `merge_commit_sha` records the resulting merge commit.

## Target model (GitHub-shaped)

These tables are *planned*, not implemented. Add them with
`cargo loco generate model <plural> ...` and put logic in the model file.

```
users                  (existing)
repositories           (existing)   + default_branch, status, org_id?, star_count
personal_access_tokens (existing)

collaborators          repository_id, user_id, role(admin|write|read)   [unique pair]

pull_requests          (existing)   + merged_commit_sha
review_comments        (existing)
pr_reviews                                                   (Phase 4, optional)
stars                  user_id, repository_id
forks                  source_repo_id, repository_id
events                 actor_id, repository_id, kind, payload(jsonb), created_at

organizations / org_members / teams        (Phase 4)
releases / tags                            (Phase 5)
```

### Rules for this model

- Issue and PR numbers are **per repository**: `unique(repository_id, number)`.
  Allocate the next number inside a transaction (a counter column on
  `repositories` is the simplest approach).
- Every table that can be reached from a URL carries a foreign key to its
  parent; cascade deletes matter for repositories and their dependants.
- Do not store commit/tree/blob data as the primary copy. If `branches` or
  `commits` are added for list performance, they are a cache synced by a task
  or worker, and every UI still tolerates a stale row.
- Authorization will be one model method, e.g. `can(user, repo, action)`, used
  by both REST handlers and `git_http`. Do not grow two permission systems.
