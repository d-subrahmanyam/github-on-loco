# Design practices

These are the conventions for this codebase. They follow the Loco doctrine
("Rails for Rust") plus the decisions in [architecture.md](architecture.md).

## Generate, then edit

Rust has no autoloading. `cargo loco generate ...` writes the file **and** its
wiring (`mod` declarations, route registration, worker/task registration, a
test file). Hand-written wiring is how "the handler exists but 404s" happens.

```sh
cargo loco generate model issues number:int title:string ...
cargo loco db migrate && cargo loco db entities
cargo loco generate controller issues --auth
cargo loco routes     # prove the endpoint is registered
```

Never hand-edit `src/models/_entities/`; it is regenerated from the live schema.

## Fat model, slim controller

- Finders and creation on `impl Model`.
- State transitions on `impl ActiveModel`, consuming `self` and returning the
  persisted `Model`.
- Validation as a named function on the model (or `Validatable` for field
  rules), called from the model's create/transition path — not duplicated as
  inline `if`s in handlers.
- Handlers parse input, call one or two model methods, render a DTO.

If a controller is 80 lines and `models/<thing>.rs` is empty, the design is
inverted.

## Use the batteries

| Need | Use | Do not |
|---|---|---|
| hash a password or token | `loco_rs::hash` | a new crypto crate |
| work after the response | `BackgroundWorker` + queue | `tokio::spawn` |
| periodic work | `Task` + `scheduler:` in config | hand-rolled sleep loop |
| email | a mailer in `src/mailers/` | a raw SMTP client |
| settings | `config/<env>.yaml` → `ctx.config` | `std::env::var` |
| validation | model rules / `Validatable` | scattered handler checks |
| a public id | the generated `pid` uuid | a home-grown id scheme |
| caching | `ctx.cache` (read its recipe first) | a global `HashMap` |

## Never put an entity on the wire

`users::Model` contains a password hash and API key. DTOs in `src/dtos/` derive
`ts-rs::TS` and export into `frontend/src/bindings/`, which makes a contract
change a compile-time event on both sides.

## Errors and status codes

- Handlers and workers return `loco_rs::Result`; use `?`, never `.unwrap()`.
- Use typed helpers: `bad_request`, `not_found`, `unauthorized`. A stringly
  typed `Error::string(...)` becomes a 500, which is only right for genuine
  internal failures.
- Multi-statement invariants run in a transaction (`db.begin()` … `commit()`).
  Check-then-insert without one is a race.

## Path and revision safety

Any string that becomes a filesystem path or a git command argument goes
through a validator first:

- repository names and owner handles: `models::repositories::*`;
- git revisions and in-tree paths: `data::git::{is_safe_ref, is_safe_path}`.

Never interpolate raw user input into a shell command. The git layer passes
arguments as a vector to `tokio::process::Command`, so there is no shell.

## Testing

Every behaviour gets a test at the lowest layer that can express it:

| Layer | Where | Example |
|---|---|---|
| Pure validation / format helpers | `#[cfg(test)]` next to the code | name and revision validators, token format |
| Model queries and transitions | `tests/models/` | issue a PAT, find a user by username |
| HTTP contract | `tests/requests/` | create repo, browse commits, reject unsafe names |
| Tasks/workers | `tests/tasks/`, `tests/workers/` | run a task with `Vars` |

Details in [testing.md](testing.md).

## Review checklist

Before calling a change done:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo loco routes     # for any new endpoint
```

Then check:

- [ ] no entity serialized straight to a response;
- [ ] no `tokio::spawn`, no `std::env::var`, no `.unwrap()` in a handler;
- [ ] no edits under `_entities/`;
- [ ] new user input validated before reaching disk or git;
- [ ] DTO exported to TypeScript if the SPA consumes it;
- [ ] a test that would fail if the change were reverted;
- [ ] docs in this folder updated when behaviour or architecture changed.
