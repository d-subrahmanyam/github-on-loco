import { FaLock, FaPlus, FaShieldAlt, FaTrash } from 'react-icons/fa'
import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { protectionRequested } from '../store/protectionSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, input, label, primaryButton } from '../ui'
import { useRepoParams } from './RepoLayout'

/** Admin-only settings for per-branch push and merge requirements. */
export function BranchProtection() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const { rules, loading, error } = useAppSelector((store) => store.protection)
  const [branch, setBranch] = useState('')
  const [requirePullRequest, setRequirePullRequest] = useState(true)
  const [requiredApprovals, setRequiredApprovals] = useState(1)

  const scope = `${owner}/${name}`

  useEffect(() => {
    dispatch(protectionRequested({ part: 'list', key: `list:${scope}`, owner, name }))
  }, [dispatch, owner, name, scope])

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    dispatch(
      protectionRequested({
        part: 'set',
        key: `set:${scope}:${branch}:${Date.now()}`,
        owner,
        name,
        params: { branch, require_pull_request: requirePullRequest, required_approvals: requiredApprovals },
      }),
    )
    setBranch('')
    setRequirePullRequest(true)
    setRequiredApprovals(1)
  }

  function remove(target: string) {
    dispatch(
      protectionRequested({
        part: 'remove',
        key: `remove:${scope}:${target}:${Date.now()}`,
        owner,
        name,
        branch: target,
      }),
    )
  }

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_340px]">
      <section className={`${card} overflow-hidden`}>
        <div className="flex items-center gap-2 border-b border-slate-200 bg-slate-50 px-5 py-4">
          <FaShieldAlt className="text-slate-500" />
          <h2 className="text-lg font-semibold">Protected branches</h2>
        </div>
        {loading.list && <p className="p-5 text-sm text-slate-500">Loading rules…</p>}
        {error.list && <p role="alert" className="p-5 text-sm text-red-700">{error.list}</p>}
        <ul className="divide-y divide-slate-100">
          {rules.length === 0 && !loading.list && (
            <li className="p-5 text-sm text-slate-500">
              No protection rules yet. Anyone with write access can push to any branch.
            </li>
          )}
          {rules.map((rule) => (
            <li key={rule.id} className="flex flex-wrap items-center gap-3 px-5 py-4">
              <FaLock className="text-amber-500" />
              <div className="flex-1">
                <p className="font-mono font-medium text-slate-800">{rule.branch}</p>
                <p className="mt-1 text-xs text-slate-500">
                  {rule.require_pull_request ? 'No direct pushes (admins exempt)' : 'Direct pushes allowed'}
                  {' · '}
                  {rule.required_approvals === 0
                    ? 'no approvals required'
                    : `${rule.required_approvals} approval${rule.required_approvals === 1 ? '' : 's'} required`}
                </p>
              </div>
              <button
                type="button"
                title={`Remove protection for ${rule.branch}`}
                onClick={() => remove(rule.branch)}
                className="rounded-lg p-2 text-red-600 hover:bg-red-50"
              >
                <FaTrash />
              </button>
            </li>
          ))}
        </ul>
        {(error.set || error.remove) && (
          <p role="alert" className="p-5 text-sm text-red-700">{error.set || error.remove}</p>
        )}
      </section>

      <aside>
        <form onSubmit={handleSubmit} className={`${card} h-fit space-y-4 p-5`}>
          <h2 className="flex items-center gap-2 text-lg font-semibold">
            <FaPlus className="text-blue-600" /> Protect a branch
          </h2>
          <div>
            <label htmlFor="protect-branch" className={label}>Branch name</label>
            <input
              id="protect-branch"
              required
              value={branch}
              onChange={(event) => setBranch(event.target.value)}
              placeholder="main"
              className={`${input} mt-1 font-mono`}
            />
          </div>
          <label className="flex items-start gap-2 text-sm text-slate-700">
            <input
              type="checkbox"
              checked={requirePullRequest}
              onChange={(event) => setRequirePullRequest(event.target.checked)}
              className="mt-1"
            />
            Require changes through a pull request
          </label>
          <div>
            <label htmlFor="required-approvals" className={label}>Required approvals</label>
            <input
              id="required-approvals"
              type="number"
              min={0}
              max={6}
              value={requiredApprovals}
              onChange={(event) => setRequiredApprovals(Number(event.target.value))}
              className={`${input} mt-1`}
            />
            <p className="mt-2 text-xs text-slate-500">
              0 disables the approval gate. Reviews must come from someone other than the PR author.
            </p>
          </div>
          <button type="submit" disabled={loading.set} className={`${primaryButton} w-full`}>
            <FaShieldAlt /> {loading.set ? 'Saving…' : 'Save rule'}
          </button>
        </form>
      </aside>
    </div>
  )
}
