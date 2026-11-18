# Notification channels

Notifications are **feature-flagged and off by default**. The app needs no mail
server or third-party credentials until you enable them. The design is a small
outbox plus a registry of interchangeable delivery channels, so adding Slack,
Teams or PagerDuty is one implementation and one registration — no schema or
call-site changes.

## How it works

```
activity event (services::events::record)
  → notifications::dispatch
      → if settings.notifications.enabled == false: return (no rows, no jobs)
      → watchers of the repo, excluding the actor
      → one pending row per (watcher, configured channel)
      → enqueue DeliverNotification worker
          → resolve channel by name from the registry
          → channel.send(...)
          → mark sent / failed (queue retries up to 3 times)
```

The activity log (`events`) and webhooks are unaffected: dispatch is
best-effort and failures are logged, never propagated to the user action.

## Configuration

`config/<environment>.yaml`:

```yaml
settings:
  notifications:
    enabled: false          # flip to true when a channel is available
    channels: ['email']     # names resolved against the channel registry
```

`enabled` can be driven by an environment variable (`NOTIFICATIONS_ENABLED`
in test/production config). When disabled, `dispatch` returns before any
database query, so there is no cost and no external dependency.

The SPA's **Notifications** page shows the current flag, configured channel
names, registered implementations and the signed-in user's delivery history.

## Built-in channels

| Name | Implementation | Notes |
|---|---|---|
| `log` | `notifications::channels::LogChannel` | Writes the notification to application logs. Safe default for local testing and useful for verifying fan-out without infrastructure. |
| `email` | `notifications::channels::EmailChannel` | Loco mailer with `src/mailers/notification/generic/*.t`. Uses `config/<env>.yaml` SMTP; `config/test.yaml` sets `mailer.stub: true`. |

## Adding a channel (Slack example)

1. Implement the trait in `src/notifications/channels.rs` (or a new module):

   ```rust
   struct SlackChannel;

   #[async_trait]
   impl NotificationChannel for SlackChannel {
       fn name(&self) -> &'static str { "slack" }

       async fn send(&self, ctx: &AppContext, n: &notifications::Model) -> Result<()> {
           // Read the webhook URL from config or a secret store, POST the
           // payload with reqwest, and return Err on non-2xx so the queue retries.
           Ok(())
       }
   }
   ```

2. Register it at startup (e.g. in `Hooks::before_run` or `after_context`):

   ```rust
   crate::notifications::register_channel(Arc::new(SlackChannel))?;
   ```

   The registry is a process-wide map keyed by `name()`. Registering the same
   name replaces the previous implementation, which also makes it easy to
   inject a fake channel in tests.

3. Add `slack` to `settings.notifications.channels`. No migration, controller
   or worker change is needed — the existing `DeliverNotification` worker
   resolves the name at delivery time.

Teams and PagerDuty follow the same pattern. PagerDuty typically wants a
severity mapping; derive it from the event name inside `send`, or extend the
`notifications` row with a `severity` column if you need it persisted.

## Delivery semantics

- Recipients are repository watchers, excluding the actor. Collaborators are
  not notified automatically; add them as watchers or extend `dispatch`.
- One row per recipient per channel makes partial success visible: if email
  fails but `log` succeeds, the SPA shows one `sent` and one `failed` row.
- `attempts` counts worker invocations; `last_error` holds the first 500
  characters of the most recent failure. Manual retry is not exposed in the SPA
  yet — the queue handles retries.
- Channel implementations should be fast and idempotent where possible. Use
  `X-GitHub-Delivery`-style dedupe keys on the receiving service if retries can
  produce duplicate side effects.

## Operational notes

- Keep webhook URLs, tokens and mail credentials in a secret store; the
  examples use config file values only for local development.
- Start with `channels: ['log']` to verify fan-out, then add `email` once SMTP
  works, then add chat/incident channels as needed.
- If many watchers exist, `dispatch` enqueues one job per watcher per channel;
  consider batching inside a channel implementation before scaling up.
