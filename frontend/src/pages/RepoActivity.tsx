import { FaHistory } from 'react-icons/fa'
import { useEffect } from 'react'
import { socialRequested } from '../store/socialSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card } from '../ui'
import { useRepoParams } from './RepoLayout'

/** Repository activity feed, built from events recorded by the API. */
export function RepoActivity() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const { events, loading, error } = useAppSelector((state) => state.social)

  useEffect(() => {
    dispatch(
      socialRequested({ part: 'events', key: `events:${owner}/${name}`, owner, name }),
    )
  }, [dispatch, owner, name])

  return (
    <section className={`${card} overflow-hidden`}>
      <div className="flex items-center gap-2 border-b border-slate-200 bg-slate-50 px-5 py-4">
        <FaHistory className="text-slate-500" />
        <h2 className="text-lg font-semibold">Activity</h2>
      </div>
      {loading.events && <p className="p-5 text-sm text-slate-500">Loading activity…</p>}
      {error.events && <p role="alert" className="p-5 text-sm text-red-700">{error.events}</p>}
      {!loading.events && events.length === 0 && (
        <p className="p-5 text-sm text-slate-500">No activity yet.</p>
      )}
      <ul className="divide-y divide-slate-100">
        {events.map((event) => (
          <li key={event.id} className="flex items-start gap-4 px-5 py-4">
            <span className="mt-2 h-2 w-2 shrink-0 rounded-full bg-blue-500" />
            <div>
              <p className="text-sm text-slate-800">
                <span className="font-medium">{event.actor}</span> {event.text}
              </p>
              <p className="mt-1 text-xs text-slate-500">
                {new Date(event.created_at).toLocaleString()}
              </p>
            </div>
          </li>
        ))}
      </ul>
    </section>
  )
}
