import { FaArrowLeft, FaClock, FaUser } from 'react-icons/fa'
import { useEffect } from 'react'
import { Link, useParams, useSearchParams } from 'react-router'
import { browseRequested } from '../store/browseSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, codeBox } from '../ui'
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
    <section className={`${card} overflow-hidden`}>
      <div className="border-b border-slate-200 bg-slate-50 px-5 py-4">
        <Link to={back} className="inline-flex items-center gap-2 text-sm text-blue-600 hover:underline">
          <FaArrowLeft /> Back to commits
        </Link>
      </div>
      {loading.detail && <p className="p-5 text-sm text-slate-500">Loading commit…</p>}
      {error.detail && <p role="alert" className="p-5 text-sm text-red-700">{error.detail}</p>}
      {detail && (
        <div className="space-y-4 p-6">
          <h2 className="whitespace-pre-wrap text-xl font-semibold">{detail.message}</h2>
          <div className="flex flex-wrap gap-4 text-sm text-slate-600">
            <span className="inline-flex items-center gap-2">
              <FaUser /> {detail.author} &lt;{detail.author_email}&gt;
            </span>
            <span className="inline-flex items-center gap-2">
              <FaClock /> {new Date(detail.committed_at).toLocaleString()}
            </span>
          </div>
          <div className={codeBox}>
            <span className="font-semibold">commit </span>
            {detail.sha}
          </div>
          {detail.parents.length > 0 && (
            <div className="flex flex-wrap items-center gap-2 text-sm text-slate-600">
              <span>Parents:</span>
              {detail.parents.map((parent) => (
                <code key={parent} className={codeBox}>{parent.slice(0, 12)}</code>
              ))}
            </div>
          )}
        </div>
      )}
    </section>
  )
}
