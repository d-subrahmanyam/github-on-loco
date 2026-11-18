import { FaCodeFork } from 'react-icons/fa6'
import {
  FaCode,
  FaCodeBranch,
  FaCopy,
  FaExclamationCircle,
  FaEye,
  FaFolder,
  FaGitAlt,
  FaHistory,
  FaRegEye,
  FaRegStar,
  FaShieldAlt,
  FaStar,
  FaUsers,
} from 'react-icons/fa'
import { useEffect, useRef, useState } from 'react'
import { NavLink, Outlet, useNavigate, useOutletContext, useParams } from 'react-router'
import { getToken } from '../auth/token'
import { browseRequested } from '../store/browseSlice'
import { forkRequested, forksReset } from '../store/forksSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { socialRequested } from '../store/socialSlice'
import { card, codeBox } from '../ui'

export interface RepoParams {
  owner: string
  name: string
}

/** Shared hook for repository child pages. */
export function useRepoParams(): RepoParams {
  return useOutletContext<RepoParams>()
}

/** The viewer's role on this repo, as reported by the overview endpoint. */
export function useRepoRole(): string {
  return useAppSelector((state) => state.browse.overview?.viewer_role ?? 'none')
}

export function canWriteRole(role: string): boolean {
  return role === 'owner' || role === 'admin' || role === 'write'
}

export function canAdminRole(role: string): boolean {
  return role === 'owner' || role === 'admin'
}

/** Header, tabs and overview data shared by every repository page. */
export function RepoLayout() {
  const dispatch = useAppDispatch()
  const navigate = useNavigate()
  const { owner = '', name = '' } = useParams()
  const { overview, loading, error } = useAppSelector((state) => state.browse)
  const social = useAppSelector((state) => state.social)
  const forks = useAppSelector((state) => state.forks)
  const stats = social.stats ?? overview
  const role = useRepoRole()
  const [copied, setCopied] = useState(false)
  const forkKey = useRef('')
  const signedIn = Boolean(getToken())

  useEffect(() => {
    dispatch(
      browseRequested({ part: 'overview', key: `${owner}/${name}`, owner, name }),
    )
    dispatch(forksReset())
  }, [dispatch, owner, name])

  // A fork creates a *different* repository, so navigate to it once the API
  // confirms success — and only for the request this component started.
  useEffect(() => {
    if (!forks.created || forks.created.key !== forkKey.current) return
    const { owner: newOwner, name: newName } = forks.created.data
    dispatch(forksReset())
    navigate(`/repos/${encodeURIComponent(newOwner)}/${encodeURIComponent(newName)}`)
  }, [dispatch, forks.created, navigate])

  function fork() {
    forkKey.current = `fork:${owner}/${name}:${Date.now()}`
    dispatch(
      forkRequested({
        part: 'create',
        key: forkKey.current,
        owner,
        name,
      }),
    )
  }

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
            {overview?.parent && (
              <p className="mt-2 text-sm text-slate-500">
                forked from{' '}
                <NavLink
                  to={`/repos/${encodeURIComponent(overview.parent.owner)}/${encodeURIComponent(overview.parent.name)}`}
                  className="font-medium text-blue-600 hover:underline"
                >
                  {overview.parent.owner}/{overview.parent.name}
                </NavLink>
              </p>
            )}
            {overview?.description && (
              <p className="mt-2 text-sm text-slate-600">{overview.description}</p>
            )}
            {loading.overview && <p className="mt-2 text-sm text-slate-400">Loading…</p>}
            {error.overview && <p role="alert" className="mt-2 text-sm text-red-700">{error.overview}</p>}
            {forks.error.create && <p role="alert" className="mt-2 text-sm text-red-700">{forks.error.create}</p>}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {signedIn && overview && (
              <button
                type="button"
                onClick={fork}
                disabled={forks.loading.create}
                title="Fork this repository to your account"
                className="inline-flex items-center gap-2 rounded-lg border border-slate-300 bg-white px-3 py-2 text-xs text-slate-700 hover:bg-slate-50 disabled:opacity-50"
              >
                <FaCodeFork /> {forks.loading.create ? 'Forking…' : 'Fork'} · {overview.fork_count}
              </button>
            )}
            {signedIn && stats && (
              <>
                <button
                  type="button"
                  onClick={() =>
                    dispatch(
                      socialRequested({
                        part: stats.starred ? 'unstar' : 'star',
                        key: `${stats.starred ? 'unstar' : 'star'}:${owner}/${name}`,
                        owner,
                        name,
                      }),
                    )
                  }
                  disabled={social.loading.star || social.loading.unstar}
                  className="inline-flex items-center gap-2 rounded-lg border border-slate-300 bg-white px-3 py-2 text-xs text-slate-700 hover:bg-slate-50 disabled:opacity-50"
                >
                  {stats.starred ? <FaStar className="text-amber-500" /> : <FaRegStar />}
                  {stats.starred ? 'Starred' : 'Star'} · {stats.star_count}
                </button>
                <button
                  type="button"
                  onClick={() =>
                    dispatch(
                      socialRequested({
                        part: stats.watching ? 'unwatch' : 'watch',
                        key: `${stats.watching ? 'unwatch' : 'watch'}:${owner}/${name}`,
                        owner,
                        name,
                      }),
                    )
                  }
                  disabled={social.loading.watch || social.loading.unwatch}
                  className="inline-flex items-center gap-2 rounded-lg border border-slate-300 bg-white px-3 py-2 text-xs text-slate-700 hover:bg-slate-50 disabled:opacity-50"
                >
                  {stats.watching ? <FaEye className="text-blue-600" /> : <FaRegEye />}
                  {stats.watching ? 'Watching' : 'Watch'} · {stats.watch_count}
                </button>
              </>
            )}
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
          <NavLink to="pulls" className={tabClass}>
            <FaGitAlt /> Pull requests
          </NavLink>
          <NavLink to="forks" className={tabClass}>
            <FaCodeFork /> Forks
          </NavLink>
          <NavLink to="activity" className={tabClass}>
            <FaHistory /> Activity
          </NavLink>
          {canAdminRole(role) && (
            <>
              <NavLink to="collaborators" className={tabClass}>
                <FaUsers /> Collaborators
              </NavLink>
              <NavLink to="branch-protection" className={tabClass}>
                <FaShieldAlt /> Branch protection
              </NavLink>
            </>
          )}
        </nav>
      </section>

      <Outlet context={{ owner, name } satisfies RepoParams} />
    </div>
  )
}
