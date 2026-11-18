import {
  FaArrowLeft,
  FaCheck,
  FaCheckCircle,
  FaCodeBranch,
  FaComment,
  FaFileAlt,
  FaGitAlt,
  FaRedo,
  FaTimesCircle,
} from 'react-icons/fa'
import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useParams } from 'react-router'
import { getToken } from '../auth/token'
import { Markdown } from '../components/Markdown'
import { protectionRequested } from '../store/protectionSlice'
import { pullRequested } from '../store/pullsSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, codeBox, input, primaryButton, secondaryButton } from '../ui'
import { canWriteRole, useRepoParams, useRepoRole } from './RepoLayout'

/** One PR: state controls, commit/file compare and review conversation. */
export function PullDetail() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const { number = '' } = useParams()
  const prNumber = Number(number)
  const { current, compare, comments, loading, error } = useAppSelector(
    (store) => store.pulls,
  )
  const { reviews, loading: protectionLoading, error: protectionError } = useAppSelector(
    (store) => store.protection,
  )
  const [tab, setTab] = useState<'conversation' | 'diff'>('conversation')
  const [reply, setReply] = useState('')

  const scope = `${owner}/${name}`
  const signedIn = Boolean(getToken())
  const canWrite = canWriteRole(useRepoRole())

  useEffect(() => {
    if (!Number.isFinite(prNumber)) return
    const keys = {
      detail: `detail:${scope}:${prNumber}`,
      comments: `comments:${scope}:${prNumber}`,
    }
    dispatch(pullRequested({ part: 'detail', key: keys.detail, owner, name, number: prNumber }))
    dispatch(pullRequested({ part: 'comments', key: keys.comments, owner, name, number: prNumber }))
    dispatch(
      protectionRequested({
        part: 'reviews',
        key: `reviews:${scope}:${prNumber}`,
        owner,
        name,
        number: prNumber,
      }),
    )
  }, [dispatch, owner, name, scope, prNumber])

  // Compare follows the head SHA stored on the PR, which the git push handler
  // refreshes whenever the head branch moves.
  useEffect(() => {
    if (!current) return
    dispatch(
      pullRequested({
        part: 'compare',
        key: `compare:${scope}:${current.number}:${current.head_sha}`,
        owner,
        name,
        head: current.head_sha,
        base: current.base_ref,
      }),
    )
  }, [dispatch, owner, name, scope, current])

  function changeState(target: 'open' | 'closed') {
    dispatch(
      pullRequested({
        part: 'state',
        key: `state:${scope}:${prNumber}:${target}`,
        owner,
        name,
        number: prNumber,
        state: target,
      }),
    )
  }

  function merge() {
    dispatch(
      pullRequested({
        part: 'merge',
        key: `merge:${scope}:${prNumber}`,
        owner,
        name,
        number: prNumber,
      }),
    )
  }

  function submitReview(state: string) {
    dispatch(
      protectionRequested({
        part: 'review',
        key: `review:${scope}:${prNumber}:${Date.now()}`,
        owner,
        name,
        number: prNumber,
        params: { state, body: null },
      }),
    )
  }

  function handleReply(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!reply.trim()) return
    dispatch(
      pullRequested({
        part: 'comment',
        key: `comment:${scope}:${prNumber}:${Date.now()}`,
        owner,
        name,
        number: prNumber,
        params: { body: reply, path: null, line: null },
      }),
    )
    setReply('')
  }

  const tabClass = (active: boolean) =>
    `border-b-2 px-4 py-3 text-sm font-medium ${
      active
        ? 'border-blue-600 text-blue-700'
        : 'border-transparent text-slate-500 hover:text-slate-800'
    }`

  return (
    <div className="space-y-6">
      <Link to="../pulls" className="inline-flex items-center gap-2 text-sm text-blue-600 hover:underline">
        <FaArrowLeft /> Back to pull requests
      </Link>

      {loading.detail && <p className="text-sm text-slate-500">Loading pull request…</p>}
      {error.detail && <p role="alert" className="text-sm text-red-700">{error.detail}</p>}

      {current && (
        <section className={`${card} p-6`}>
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div>
              <h2 className="flex flex-wrap items-center gap-2 text-2xl font-semibold">
                <span
                  className={`rounded-full px-2 py-1 text-xs font-medium ${
                    current.state === 'open'
                      ? 'bg-green-100 text-green-800'
                      : current.state === 'merged'
                        ? 'bg-purple-100 text-purple-800'
                        : 'bg-slate-200 text-slate-700'
                  }`}
                >
                  {current.state}
                </span>
                {current.title}
              </h2>
              <p className="mt-2 text-sm text-slate-500">
                #{current.number} opened by {current.author}
              </p>
              <p className={`${codeBox} mt-3 inline-flex items-center gap-2`}>
                <FaCodeBranch /> {current.head_ref} <span className="text-slate-400">→</span>{' '}
                {current.base_ref}
              </p>
              {current.required_approvals > 0 && (
                <p className="mt-3 inline-flex items-center gap-2 rounded-lg bg-slate-100 px-3 py-1.5 text-sm text-slate-700">
                  <FaCheckCircle className="text-green-600" />
                  {current.approval_count} of {current.required_approvals} approvals
                </p>
              )}
            </div>
            {canWrite && current.state === 'open' && (
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  disabled={loading.state || loading.merge}
                  onClick={() => changeState('closed')}
                  className={`${secondaryButton} disabled:opacity-50`}
                >
                  <FaCheck /> Close
                </button>
                <button
                  type="button"
                  disabled={loading.merge || loading.state || Boolean(current.merge_blocked_reason)}
                  onClick={merge}
                  title={current.merge_blocked_reason ?? undefined}
                  className={`${primaryButton} disabled:opacity-50`}
                >
                  <FaGitAlt /> {loading.merge ? 'Merging…' : 'Merge pull request'}
                </button>
              </div>
            )}
            {canWrite && current.state === 'closed' && (
              <button
                type="button"
                disabled={loading.state}
                onClick={() => changeState('open')}
                className={`${primaryButton} disabled:opacity-50`}
              >
                <FaRedo /> Reopen
              </button>
            )}
          </div>
          {current.state === 'open' && current.merge_blocked_reason && (
            <p className="mt-4 rounded-lg bg-amber-50 px-3 py-2 text-sm text-amber-800">
              <FaTimesCircle className="mr-2 inline" />
              Merge blocked: {current.merge_blocked_reason}
            </p>
          )}
          {current.merged_commit_sha && (
            <p className="mt-4 text-sm text-green-800">
              Merged in <code className="font-mono">{current.merged_commit_sha.slice(0, 12)}</code>
            </p>
          )}
          {(error.merge || error.state) && (
            <p role="alert" className="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">
              {error.merge || error.state}
            </p>
          )}
          {current.body ? (
            <Markdown source={current.body} className="mt-6" />
          ) : (
            <p className="mt-6 text-slate-400">No description provided.</p>
          )}
        </section>
      )}

      <section className={`${card} overflow-hidden`}>
        <nav className="flex border-b border-slate-200 px-2">
          <button type="button" className={tabClass(tab === 'conversation')} onClick={() => setTab('conversation')}>
            <span className="inline-flex items-center gap-2"><FaComment /> Conversation ({comments.length})</span>
          </button>
          <button type="button" className={tabClass(tab === 'diff')} onClick={() => setTab('diff')}>
            <span className="inline-flex items-center gap-2"><FaFileAlt /> Commits &amp; files</span>
          </button>
        </nav>

        {tab === 'conversation' && (
          <>
            <div className="border-b border-slate-200 px-5 py-4">
              <h3 className="mb-3 text-sm font-semibold text-slate-700">
                Reviews ({reviews.length})
              </h3>
              {protectionLoading.reviews && (
                <p className="text-sm text-slate-500">Loading reviews…</p>
              )}
              {protectionError.review && (
                <p role="alert" className="text-sm text-red-700">{protectionError.review}</p>
              )}
              <ul className="space-y-2">
                {reviews.map((review) => (
                  <li key={review.id} className="flex flex-wrap items-center gap-2 text-sm">
                    {review.state === 'approved' ? (
                      <FaCheckCircle className="text-green-600" />
                    ) : review.state === 'changes_requested' ? (
                      <FaTimesCircle className="text-red-600" />
                    ) : (
                      <FaComment className="text-slate-400" />
                    )}
                    <span className="font-medium text-slate-800">{review.author}</span>
                    <span className="text-slate-600">{review.state.replace('_', ' ')}</span>
                    {review.body && (
                      <span className="w-full">
                        <Markdown source={review.body} />
                      </span>
                    )}
                  </li>
                ))}
              </ul>
              {signedIn && canWrite && current?.state === 'open' && (
                <div className="mt-4 flex flex-wrap gap-2">
                  <button
                    type="button"
                    disabled={protectionLoading.review}
                    onClick={() => submitReview('approved')}
                    className={secondaryButton}
                  >
                    <FaCheckCircle /> Approve
                  </button>
                  <button
                    type="button"
                    disabled={protectionLoading.review}
                    onClick={() => submitReview('changes_requested')}
                    className={secondaryButton}
                  >
                    <FaTimesCircle /> Request changes
                  </button>
                  <button
                    type="button"
                    disabled={protectionLoading.review}
                    onClick={() => submitReview('commented')}
                    className={secondaryButton}
                  >
                    <FaComment /> Comment only
                  </button>
                </div>
              )}
            </div>
            {loading.comments && <p className="p-5 text-sm text-slate-500">Loading comments…</p>}
            <ul className="divide-y divide-slate-100">
              {comments.map((comment) => (
                <li key={comment.id} className="px-5 py-4">
                  <p className="text-sm font-medium text-slate-800">{comment.author}</p>
                  {comment.path && (
                    <p className="mt-1 font-mono text-xs text-slate-500">
                      {comment.path}{comment.line ? `:${comment.line}` : ''}
                    </p>
                  )}
                  <Markdown source={comment.body} className="mt-2" />
                </li>
              ))}
            </ul>
            {signedIn ? (
              <form onSubmit={handleReply} className="space-y-3 border-t border-slate-200 p-5">
                <textarea
                  required
                  rows={3}
                  value={reply}
                  onChange={(event) => setReply(event.target.value)}
                  placeholder="Leave a review comment"
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
          </>
        )}

        {tab === 'diff' && (
          <div className="space-y-4 p-5">
            {loading.compare && <p className="text-sm text-slate-500">Loading comparison…</p>}
            {error.compare && <p role="alert" className="text-sm text-red-700">{error.compare}</p>}
            {compare && (
              <>
                <p className="text-sm text-slate-600">
                  <span className="font-medium text-green-700">{compare.ahead_by} ahead</span>
                  {' · '}
                  <span className="font-medium text-red-700">{compare.behind_by} behind</span>
                </p>
                <div>
                  <h3 className="mb-2 text-sm font-semibold text-slate-700">
                    Commits ({compare.commits.length})
                  </h3>
                  <ul className="divide-y divide-slate-100 rounded-lg border border-slate-200">
                    {compare.commits.map((commit) => (
                      <li key={commit.sha} className="px-3 py-2">
                        <p className="text-sm text-slate-800">{commit.message}</p>
                        <p className="font-mono text-xs text-slate-500">
                          {commit.sha.slice(0, 8)} · {commit.author}
                        </p>
                      </li>
                    ))}
                  </ul>
                </div>
                <div>
                  <h3 className="mb-2 text-sm font-semibold text-slate-700">
                    Changed files ({compare.files.length})
                  </h3>
                  <div className="space-y-2">
                    {compare.files.map((file) => (
                      <details key={file.path} className={`${card} overflow-hidden`}>
                        <summary className="cursor-pointer px-4 py-3 text-sm">
                          <span className="font-mono text-slate-800">{file.path}</span>
                          <span className="ml-2 rounded bg-slate-100 px-1.5 py-0.5 text-xs text-slate-500">
                            {file.status}
                          </span>
                          <span className="ml-2 text-xs text-green-700">+{file.additions}</span>
                          <span className="ml-1 text-xs text-red-700">−{file.deletions}</span>
                        </summary>
                        {file.patch && (
                          <pre className="overflow-x-auto border-t border-slate-200 bg-slate-50 p-3 text-xs leading-relaxed">
                            {file.patch}
                          </pre>
                        )}
                      </details>
                    ))}
                  </div>
                </div>
              </>
            )}
          </div>
        )}
      </section>
    </div>
  )
}
