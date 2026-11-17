import {
  FaCode,
  FaCodeBranch,
  FaCopy,
  FaExclamationCircle,
  FaFolder,
  FaHistory,
} from 'react-icons/fa'
import { useEffect, useState } from 'react'
import { NavLink, Outlet, useOutletContext, useParams } from 'react-router'
import { browseRequested } from '../store/browseSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, codeBox } from '../ui'

export interface RepoParams {
  owner: string
  name: string
}

/** Shared hook for repository child pages. */
export function useRepoParams(): RepoParams {
  return useOutletContext<RepoParams>()
}

/** Header, tabs and overview data shared by every repository page. */
export function RepoLayout() {
  const dispatch = useAppDispatch()
  const { owner = '', name = '' } = useParams()
  const { overview, loading, error } = useAppSelector((state) => state.browse)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    dispatch(
      browseRequested({ part: 'overview', key: `${owner}/${name}`, owner, name }),
    )
  }, [dispatch, owner, name])

  async function copyCloneUrl() {
    if (!overview) return
    await navigator.clipboard.writeText(overview.clone_url)
    setCopied(true)
    window.setTimeout(() => setCopied(false), 1500)
  }

  const tabClass = ({ isActive }: { isActive: boolean }) =>
    `flex items-center gap-2 border-b-2 px-4 py-3 text-sm font-medium transition-colors ${
      isActive
        ? 'border-blue-600 text-blue-700'
        : 'border-transparent text-slate-500 hover:border-slate-300 hover:text-slate-800'
    }`

  return (
    <div className="space-y-6">
      <section className={card}>
        <div className="flex flex-wrap items-start justify-between gap-4 p-6">
          <div>
            <h1 className="flex flex-wrap items-center gap-2 text-2xl font-semibold">
              <FaFolder className="text-slate-800" />
              <span className="text-slate-500">{owner}</span>
              <span>/</span>
              <span>{name}</span>
            </h1>
            {overview?.description && (
              <p className="mt-2 text-sm text-slate-600">{overview.description}</p>
            )}
            {loading.overview && <p className="mt-2 text-sm text-slate-400">Loading…</p>}
            {error.overview && <p role="alert" className="mt-2 text-sm text-red-700">{error.overview}</p>}
          </div>
          {overview && (
            <button
              type="button"
              onClick={copyCloneUrl}
              title="Copy clone URL"
              className="inline-flex items-center gap-2 rounded-lg border border-slate-300 bg-white px-3 py-2 text-xs text-slate-700 hover:bg-slate-50"
            >
              <FaCopy /> {copied ? 'Copied!' : 'Clone URL'}
            </button>
          )}
        </div>
        {overview && <div className={`${codeBox} m-6 mt-0 truncate`}>{overview.clone_url}</div>}

        <nav className="flex gap-1 overflow-x-auto border-t border-slate-200 px-2">
          <NavLink end to="" className={tabClass}>
            <FaCode /> Code
          </NavLink>
          <NavLink to="commits" className={tabClass}>
            <FaHistory /> Commits
          </NavLink>
          <NavLink to="branches" className={tabClass}>
            <FaCodeBranch /> Branches
          </NavLink>
          <NavLink to="issues" className={tabClass}>
            <FaExclamationCircle /> Issues
          </NavLink>
        </nav>
      </section>

      <Outlet context={{ owner, name } satisfies RepoParams} />
    </div>
  )
}
