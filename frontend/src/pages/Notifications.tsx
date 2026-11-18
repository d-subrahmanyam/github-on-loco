import { FaBell, FaCheckCircle, FaTimesCircle } from 'react-icons/fa'
import { useEffect } from 'react'
import { notificationRequested } from '../store/notificationsSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card } from '../ui'

/** Delivery history and the current feature-flag/channel configuration. */
export function Notifications() {
  const dispatch = useAppDispatch()
  const { items, settings, loading, error } = useAppSelector((state) => state.notifications)

  useEffect(() => {
    dispatch(notificationRequested({ part: 'list', key: 'notifications:list' }))
    dispatch(notificationRequested({ part: 'settings', key: 'notifications:settings' }))
  }, [dispatch])

  return (
    <div className="mx-auto max-w-3xl space-y-6">
      <section className={`${card} p-5`}>
        <h2 className="flex items-center gap-2 text-lg font-semibold">
          <FaBell className="text-blue-600" /> Notifications
        </h2>
        {loading.settings && <p className="mt-2 text-sm text-slate-500">Loading settings…</p>}
        {settings && (
          <div className="mt-3 space-y-2 text-sm text-slate-700">
            <p>
              Status:{' '}
              <span className={settings.enabled ? 'text-green-700' : 'text-amber-700'}>
                {settings.enabled ? 'enabled' : 'disabled'}
              </span>
            </p>
            <p>Configured channels: {settings.channels.join(', ') || 'none'}</p>
            <p>Available implementations: {settings.available.join(', ') || 'none'}</p>
            {!settings.enabled && (
              <p className="rounded-lg bg-amber-50 p-3 text-amber-800">
                Notifications are off. Enable <code>settings.notifications</code> in the server
                config once an email server or another channel is available.
              </p>
            )}
          </div>
        )}
        {error.settings && <p role="alert" className="mt-2 text-sm text-red-700">{error.settings}</p>}
      </section>

      <section className={`${card} overflow-hidden`}>
        <div className="border-b border-slate-200 bg-slate-50 px-5 py-4">
          <h2 className="text-lg font-semibold">Recent deliveries</h2>
        </div>
        {loading.list && <p className="p-5 text-sm text-slate-500">Loading…</p>}
        {error.list && <p role="alert" className="p-5 text-sm text-red-700">{error.list}</p>}
        {!loading.list && items.length === 0 && (
          <p className="p-5 text-sm text-slate-500">No notifications yet.</p>
        )}
        <ul className="divide-y divide-slate-100">
          {items.map((item) => (
            <li key={item.id} className="space-y-1 px-5 py-4">
              <div className="flex items-center justify-between gap-3">
                <p className="font-medium text-slate-800">{item.title}</p>
                <span
                  className={
                    item.status === 'sent' ? 'text-green-700' : 'text-red-700'
                  }
                  title={`${item.status} after ${item.attempts} attempt(s)`}
                >
                  {item.status === 'sent' ? <FaCheckCircle /> : <FaTimesCircle />}
                </span>
              </div>
              <p className="whitespace-pre-wrap text-sm text-slate-600">{item.body}</p>
              <p className="text-xs text-slate-500">
                {item.event} · {item.channel} · {item.created_at.slice(0, 16).replace('T', ' ')}
                {item.status === 'failed' && item.attempts > 0 ? ` · ${item.attempts} attempt(s)` : ''}
              </p>
            </li>
          ))}
        </ul>
      </section>
    </div>
  )
}
