import { useEffect } from 'react'
import { Link, useSearchParams } from 'react-router'
import { browseRequested } from '../store/browseSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
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
    <div>
      <h2>Commits{ref ? ` on ${ref}` : ''}</h2>
      {loading.commits && <p>Loading commits…</p>}
      {error.commits && <p role="alert">{error.commits}</p>}
      <ul>
        {commits.map((commit) => (
          <li key={commit.sha}>
            <Link
              to={`commit/${commit.sha}${ref ? `?ref=${encodeURIComponent(ref)}` : ''}`}
            >
              {commit.message}
            </Link>
            <div>
              <small>
                {commit.author} · {new Date(commit.committed_at).toLocaleString()}
              </small>
            </div>
          </li>
        ))}
      </ul>
    </div>
  )
}
