import { FaArrowLeft, FaCheck, FaComment, FaExclamationCircle, FaRedo, FaTag } from 'react-icons/fa'
import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useParams } from 'react-router'
import { getToken } from '../auth/token'
import { issueRequested } from '../store/issuesSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, input, primaryButton } from '../ui'
import { canWriteRole, useRepoParams, useRepoRole } from './RepoLayout'

/** One issue: state controls, labels, comments and a reply form. */
export function IssueDetail() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const { number = '' } = useParams()
  const issueNumber = Number(number)
  const { current, comments, loading, error } = useAppSelector((state) => state.issues)
  const [reply, setReply] = useState('')

  const scope = `${owner}/${name}`
  const canWrite = canWriteRole(useRepoRole())

  useEffect(() => {
    if (!Number.isFinite(issueNumber)) return
    dispatch(
      issueRequested({
        part: 'detail',
        key: `detail:${scope}:${issueNumber}`,
        owner,
        name,
        number: issueNumber,
      }),
    )
    dispatch(
      issueRequested({
        part: 'comments',
        key: `comments:${scope}:${issueNumber}`,
        owner,
        name,
        number: issueNumber,
      }),
    )
  }, [dispatch, owner, name, scope, issueNumber])

  function changeState(target: 'open' | 'closed') {
    dispatch(
      issueRequested({
        part: 'state',
        key: `state:${scope}:${issueNumber}:${target}`,
        owner,
        name,
        number: issueNumber,
        state: target,
      }),
    )
  }

  function handleReply(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!reply.trim()) return
    dispatch(
      issueRequested({
        part: 'comment',
        key: `comment:${scope}:${issueNumber}:${Date.now()}`,
        owner,
        name,
        number: issueNumber,
        params: { body: reply },
      }),
    )
    setReply('')
  }

  return (
    <div className="space-y-6">
      <Link to="../issues" className="inline-flex items-center gap-2 text-sm text-blue-600 hover:underline">
        <FaArrowLeft /> Back to issues
      </Link>

      {loading.detail && <p className="text-sm text-slate-500">Loading issue…</p>}
      {error.detail && <p role="alert" className="text-sm text-red-700">{error.detail}</p>}

      {current && (
        <section className={`${card} p-6`}>
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div>
              <h2 className="flex items-center gap-2 text-2xl font-semibold">
                <FaExclamationCircle className="text-slate-700" />
                {current.title}
              </h2>
              <p className="mt-1 text-sm text-slate-500">
                #{current.number} opened by {current.author}
              </p>
            </div>
            {canWrite && (
              <button
                type="button"
                disabled={loading.state}
                onClick={() => changeState(current.state === 'open' ? 'closed' : 'open')}
                className={
                  current.state === 'open'
                    ? 'inline-flex items-center gap-2 rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-50'
                    : `${primaryButton} disabled:opacity-50`
                }
              >
                {current.state === 'open' ? <FaCheck /> : <FaRedo />}
                {current.state === 'open' ? 'Close issue' : 'Reopen issue'}
              </button>
            )}
          </div>

          {current.labels.length > 0 && (
            <div className="mt-4 flex flex-wrap gap-2">
              {current.labels.map((item) => (
                <span
                  key={item.id}
                  className="inline-flex items-center gap-1 rounded-full bg-slate-100 px-2 py-1 text-xs text-slate-700"
                >
                  <FaTag style={{ color: `#${item.color}` }} />
                  {item.name}
                </span>
              ))}
            </div>
          )}

          <p className="mt-6 whitespace-pre-wrap text-slate-700">
            {current.body || <em className="text-slate-400">No description provided.</em>}
          </p>
        </section>
      )}

      <section className={`${card} overflow-hidden`}>
        <div className="flex items-center gap-2 border-b border-slate-200 bg-slate-50 px-5 py-4">
          <FaComment className="text-slate-500" />
          <h3 className="text-lg font-semibold">Comments ({comments.length})</h3>
        </div>
        {loading.comments && <p className="p-5 text-sm text-slate-500">Loading comments…</p>}
        <ul className="divide-y divide-slate-100">
          {comments.map((comment) => (
            <li key={comment.id} className="px-5 py-4">
              <p className="text-sm font-medium text-slate-800">{comment.author}</p>
              <p className="mt-2 whitespace-pre-wrap text-sm text-slate-700">{comment.body}</p>
            </li>
          ))}
        </ul>
        {getToken() ? (
          <form onSubmit={handleReply} className="space-y-3 border-t border-slate-200 p-5">
            <textarea
              required
              rows={3}
              value={reply}
              onChange={(event) => setReply(event.target.value)}
              placeholder="Leave a comment"
              className={input}
            />
            <button type="submit" disabled={loading.comment} className={primaryButton}>
              <FaComment /> {loading.comment ? 'Posting…' : 'Comment'}
            </button>
            {error.comment && <p role="alert" className="text-sm text-red-700">{error.comment}</p>}
          </form>
        ) : (
          <p className="p-5 text-sm text-slate-500">
            <Link to="/login" className="text-blue-600 hover:underline">Log in</Link> to comment.
          </p>
        )}
      </section>
    </div>
  )
}
