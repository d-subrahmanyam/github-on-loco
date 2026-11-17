import { FaCodeBranch } from 'react-icons/fa'
import { useEffect } from 'react'
import { Link } from 'react-router'
import { browseRequested } from '../store/browseSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card } from '../ui'
import { useRepoParams } from './RepoLayout'

/** Branch list with links to each branch's code and commit history. */
export function Branches() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const { branches, loading, error } = useAppSelector((state) => state.browse)

  useEffect(() => {
    dispatch(
      browseRequested({ part: 'branches', key: `${owner}/${name}`, owner, name }),
    )
  }, [dispatch, owner, name])

  return (
    <section className={`${card} overflow-hidden`}>
      <div className="flex items-center gap-2 border-b border-slate-200 bg-slate-50 px-5 py-4">
        <FaCodeBranch className="text-slate-500" />
        <h2 className="text-lg font-semibold">Branches</h2>
      </div>
      {loading.branches && <p className="p-5 text-sm text-slate-500">Loading branches…</p>}
      {error.branches && <p role="alert" className="p-5 text-sm text-red-700">{error.branches}</p>}
      <ul className="divide-y divide-slate-100">
        {branches.length === 0 && !loading.branches && (
          <li className="p-5 text-sm text-slate-500">No branches found.</li>
        )}
        {branches.map((branch) => (
          <li key={branch.name} className="flex items-center gap-3 px-5 py-4 hover:bg-slate-50">
            <FaCodeBranch className="text-slate-400" />
            <Link
              to={`?ref=${encodeURIComponent(branch.name)}`}
              className="font-medium text-blue-600 hover:underline"
            >
              {branch.name}
            </Link>
            <code className="ml-auto rounded bg-slate-100 px-2 py-1 text-xs text-slate-600">
              {branch.sha.slice(0, 12)}
            </code>
          </li>
        ))}
      </ul>
    </section>
  )
}
