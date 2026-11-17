import { useEffect } from 'react'
import { Link, useParams, useSearchParams } from 'react-router'
import { browseRequested } from '../store/browseSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { useRepoParams } from './RepoLayout'

/** Full message and metadata of one commit. */
export function CommitDetail() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const { sha = '' } = useParams()
  const [searchParams] = useSearchParams()
  const ref = searchParams.get('ref') ?? undefined
  const { detail, loading, error } = useAppSelector((state) => state.browse)

  useEffect(() => {
    dispatch(
      browseRequested({
        part: 'detail',
        key: `${owner}/${name}#${sha}`,
        owner,
        name,
        sha,
      }),
    )
  }, [dispatch, owner, name, sha])

  const back = ref ? `../commits?ref=${encodeURIComponent(ref)}` : '../commits'

  return (
    <div>
      <Link to={back}>← Back to commits</Link>
      {loading.detail && <p>Loading commit…</p>}
      {error.detail && <p role="alert">{error.detail}</p>}
      {detail && (
        <article>
          <h2>{detail.message}</h2>
          <p>
            <small>
              {detail.author} &lt;{detail.author_email}&gt; ·{' '}
              {new Date(detail.committed_at).toLocaleString()}
            </small>
          </p>
          <p>
            Commit <code>{detail.sha}</code>
          </p>
          {detail.parents.length > 0 && (
            <p>
              Parents:{' '}
              {detail.parents.map((parent) => (
                <code key={parent}>{parent} </code>
              ))}
            </p>
          )}
        </article>
      )}
    </div>
  )
}
