import { FaExternalLinkAlt } from 'react-icons/fa'
import { FaCodeFork } from 'react-icons/fa6'
import { useEffect } from 'react'
import { Link } from 'react-router'
import { forkRequested } from '../store/forksSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card } from '../ui'
import { useRepoParams } from './RepoLayout'

/** Public list of repositories forked from this one. */
export function Forks() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const { forks, loading, error } = useAppSelector((state) => state.forks)

  useEffect(() => {
    dispatch(
      forkRequested({
        part: 'list',
        key: `list:${owner}/${name}`,
        owner,
        name,
      }),
    )
  }, [dispatch, owner, name])

  return (
    <section className={`${card} overflow-hidden`}>
      <div className="flex items-center gap-2 border-b border-slate-200 bg-slate-50 px-5 py-4">
        <FaCodeFork className="text-slate-500" />
        <h2 className="text-lg font-semibold">Forks</h2>
      </div>
      {loading.list && <p className="p-5 text-sm text-slate-500">Loading forks…</p>}
      {error.list && <p role="alert" className="p-5 text-sm text-red-700">{error.list}</p>}
      {!loading.list && forks.length === 0 && (
        <p className="p-5 text-sm text-slate-500">No forks yet.</p>
      )}
      <ul className="divide-y divide-slate-100">
        {forks.map((fork) => (
          <li key={fork.id} className="flex items-center justify-between gap-4 px-5 py-4">
            <div>
              <Link
                to={`/repos/${encodeURIComponent(fork.owner)}/${encodeURIComponent(fork.name)}`}
                className="inline-flex items-center gap-2 font-medium text-blue-600 hover:underline"
              >
                <FaCodeFork /> {fork.owner}/{fork.name}
              </Link>
              <p className="mt-1 text-xs text-slate-500">
                forked {new Date(fork.created_at).toLocaleDateString()}
              </p>
            </div>
            <FaExternalLinkAlt className="text-slate-300" />
          </li>
        ))}
      </ul>
    </section>
  )
}
