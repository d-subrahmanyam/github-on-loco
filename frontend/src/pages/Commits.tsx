import { FaClock, FaHistory, FaUser } from 'react-icons/fa'
import { useEffect } from 'react'
import { Link, useSearchParams } from 'react-router'
import { browseRequested } from '../store/browseSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card } from '../ui'
import { useRepoParams } from './RepoLayout'

/** Commit history for the selected revision. */
export function Commits() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const [searchParams] = useSearchParams()
  const ref = searchParams.get('ref') ?? undefined
  const { commits, loading, error } = useAppSelector((state) => state.browse)

  useEffect(() => {
    dispatch(
      browseRequested({
        part: 'commits',
        key: `${owner}/${name}@${ref ?? 'default'}`,
        owner,
        name,
        ref,
      }),
    )
  }, [dispatch, owner, name, ref])

  return (
    <section className={`${card} overflow-hidden`}>
      <div className="flex items-center gap-2 border-b border-slate-200 bg-slate-50 px-5 py-4">
        <FaHistory className="text-slate-500" />
        <h2 className="text-lg font-semibold">
          Commits{ref ? <span className="font-normal text-slate-500"> on {ref}</span> : null}
        </h2>
      </div>
      {loading.commits && <p className="p-5 text-sm text-slate-500">Loading commits…</p>}
      {error.commits && <p role="alert" className="p-5 text-sm text-red-700">{error.commits}</p>}
      <ul className="divide-y divide-slate-100">
        {commits.length === 0 && !loading.commits && (
          <li className="p-5 text-sm text-slate-500">No commits yet.</li>
        )}
        {commits.map((commit) => (
          <li key={commit.sha} className="px-5 py-4 hover:bg-slate-50">
            <Link
              to={`commit/${commit.sha}${ref ? `?ref=${encodeURIComponent(ref)}` : ''}`}
              className="font-medium text-blue-600 hover:underline"
            >
              {commit.message}
            </Link>
            <div className="mt-1 flex flex-wrap gap-4 text-xs text-slate-500">
              <span className="inline-flex items-center gap-1">
                <FaUser /> {commit.author}
              </span>
              <span className="inline-flex items-center gap-1">
                <FaClock /> {new Date(commit.committed_at).toLocaleString()}
              </span>
              <span className="font-mono">{commit.sha.slice(0, 8)}</span>
            </div>
          </li>
        ))}
      </ul>
    </section>
  )
}
