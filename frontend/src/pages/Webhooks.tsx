import { FaKey, FaPlus, FaTrash } from 'react-icons/fa6'
import { FaPlug, FaRedo } from 'react-icons/fa'
import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import type { WebhookResponse } from '../bindings/WebhookResponse'
import { webhookRequested } from '../store/webhooksSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, input, label, primaryButton } from '../ui'
import { canAdminRole, useRepoParams, useRepoRole } from './RepoLayout'

/** Admin-only webhook management and delivery history. */
export function Webhooks() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const role = useRepoRole()
  const { webhooks, kinds, deliveries, loading, error, createdSecret } = useAppSelector(
    (state) => state.webhooks,
  )
  const [url, setUrl] = useState('')
  const [events, setEvents] = useState<string[]>([])
  const [selected, setSelected] = useState<number | null>(null)

  const key = `${owner}/${name}`

  useEffect(() => {
    if (!canAdminRole(role)) return
    dispatch(webhookRequested({ part: 'list', key: `webhooks:${key}`, owner, name }))
    dispatch(webhookRequested({ part: 'kinds', key: 'webhook-kinds', owner, name }))
  }, [dispatch, owner, name, role, key])

  function loadDeliveries(id: number) {
    setSelected(id)
    dispatch(
      webhookRequested({
        part: 'deliveries',
        key: `deliveries:${key}:${id}`,
        owner,
        name,
        id,
      }),
    )
  }

  function toggleEvent(eventName: string) {
    setEvents((current) =>
      current.includes(eventName)
        ? current.filter((item) => item !== eventName)
        : [...current, eventName],
    )
  }

  function handleCreate(submitEvent: FormEvent<HTMLFormElement>) {
    submitEvent.preventDefault()
    dispatch(
      webhookRequested({
        part: 'create',
        key: `create-webhook:${key}:${Date.now()}`,
        owner,
        name,
        params: { url, events: events.join(',') },
      }),
    )
    setUrl('')
    setEvents([])
  }

  if (!canAdminRole(role)) {
    return <p className="p-5 text-sm text-slate-500">Only repository admins can manage webhooks.</p>
  }

  return (
    <div className="space-y-6">
      <form onSubmit={handleCreate} className={`${card} space-y-4 p-5`}>
        <h2 className="flex items-center gap-2 text-lg font-semibold">
          <FaPlug className="text-blue-600" /> Add webhook
        </h2>
        <div>
          <label htmlFor="webhook-url" className={label}>Payload URL</label>
          <input
            id="webhook-url"
            required
            type="url"
            value={url}
            onChange={(event) => setUrl(event.target.value)}
            placeholder="https://example.com/hook"
            className={`${input} mt-1`}
          />
        </div>
        <fieldset>
          <legend className={label}>Events</legend>
          <div className="mt-2 grid gap-2 sm:grid-cols-3">
            {['*', ...kinds].map((kind) => (
              <label key={kind} className="flex items-center gap-1.5 text-sm text-slate-700">
                <input
                  type="checkbox"
                  checked={events.includes(kind)}
                  onChange={() => toggleEvent(kind)}
                />
                <span className="font-mono text-xs">{kind}</span>
              </label>
            ))}
          </div>
        </fieldset>
        <button type="submit" disabled={loading.create || events.length === 0} className={primaryButton}>
          <FaPlus /> {loading.create ? 'Adding…' : 'Add webhook'}
        </button>
        {error.create && <p role="alert" className="text-sm text-red-700">{error.create}</p>}
      </form>

      {createdSecret && (
        <div role="status" className={`${card} border-amber-300 bg-amber-50 p-5`}>
          <p className="flex items-center gap-2 font-medium text-amber-900">
            <FaKey /> Copy this signing secret now — it is shown only once
          </p>
          <code className="mt-2 block break-all rounded bg-amber-100 p-2 font-mono text-xs text-amber-900">
            {createdSecret.secret}
          </code>
        </div>
      )}

      <section className={`${card} overflow-hidden`}>
        <div className="border-b border-slate-200 bg-slate-50 px-5 py-4">
          <h2 className="text-lg font-semibold">Webhooks ({webhooks.length})</h2>
        </div>
        {loading.list && <p className="p-5 text-sm text-slate-500">Loading…</p>}
        {error.list && <p role="alert" className="p-5 text-sm text-red-700">{error.list}</p>}
        {!loading.list && webhooks.length === 0 && (
          <p className="p-5 text-sm text-slate-500">No webhooks configured.</p>
        )}
        <ul className="divide-y divide-slate-100">
          {webhooks.map((hook: WebhookResponse) => (
            <li key={hook.id} className="space-y-3 p-5">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div>
                  <p className="break-all font-mono text-sm">{hook.url}</p>
                  <p className="mt-1 text-xs text-slate-500">
                    {hook.events} · by {hook.created_by} · {hook.is_active ? 'active' : 'inactive'}
                  </p>
                </div>
                <div className="flex flex-wrap items-center gap-3">
                  <button
                    type="button"
                    className="text-sm text-blue-600 hover:underline"
                    onClick={() => loadDeliveries(hook.id)}
                  >
                    Deliveries
                  </button>
                  <button
                    type="button"
                    className="text-sm text-blue-600 hover:underline"
                    onClick={() =>
                      dispatch(
                        webhookRequested({
                          part: 'rotate',
                          key: `rotate:${hook.id}:${Date.now()}`,
                          owner,
                          name,
                          id: hook.id,
                        }),
                      )
                    }
                  >
                    Rotate secret
                  </button>
                  <button
                    type="button"
                    className="text-sm text-blue-600 hover:underline"
                    onClick={() =>
                      dispatch(
                        webhookRequested({
                          part: 'update',
                          key: `toggle:${hook.id}:${Date.now()}`,
                          owner,
                          name,
                          id: hook.id,
                          params: { events: hook.events, is_active: !hook.is_active },
                        }),
                      )
                    }
                  >
                    {hook.is_active ? 'Disable' : 'Enable'}
                  </button>
                  <button
                    type="button"
                    className="inline-flex items-center gap-1 text-sm text-red-600 hover:underline"
                    onClick={() =>
                      dispatch(
                        webhookRequested({
                          part: 'remove',
                          key: `remove-webhook:${hook.id}:${Date.now()}`,
                          owner,
                          name,
                          id: hook.id,
                        }),
                      )
                    }
                  >
                    <FaTrash /> Delete
                  </button>
                </div>
              </div>
              {selected === hook.id && (
                <div className="rounded-lg bg-slate-50 p-3">
                  <p className="mb-2 text-xs font-semibold uppercase text-slate-500">
                    Recent deliveries
                  </p>
                  {deliveries[hook.id]?.length ? (
                    <ul className="space-y-2">
                      {deliveries[hook.id].map((delivery) => (
                        <li
                          key={delivery.id}
                          className="flex flex-wrap items-center justify-between gap-2 text-sm"
                        >
                          <span>
                            <span className="font-mono">{delivery.event}</span> ·{' '}
                            <span
                              className={
                                delivery.status === 'success'
                                  ? 'text-green-700'
                                  : 'text-red-700'
                              }
                            >
                              {delivery.status}
                            </span>{' '}
                            · {delivery.attempts} attempt(s)
                            {delivery.last_error && ` · ${delivery.last_error}`}
                          </span>
                          <button
                            type="button"
                            title="Retry delivery"
                            className="text-blue-600"
                            onClick={() =>
                              dispatch(
                                webhookRequested({
                                  part: 'retry',
                                  key: `retry:${delivery.id}:${Date.now()}`,
                                  owner,
                                  name,
                                  id: hook.id,
                                  deliveryId: delivery.id,
                                }),
                              )
                            }
                          >
                            <FaRedo />
                          </button>
                        </li>
                      ))}
                    </ul>
                  ) : (
                    <p className="text-sm text-slate-400">No deliveries yet.</p>
                  )}
                  {error.deliveries && (
                    <p role="alert" className="mt-2 text-sm text-red-700">{error.deliveries}</p>
                  )}
                </div>
              )}
            </li>
          ))}
        </ul>
      </section>
    </div>
  )
}
