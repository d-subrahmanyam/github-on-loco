import { useEffect } from 'react'
import { NavLink, Outlet, useOutletContext, useParams } from 'react-router'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { browseRequested } from '../store/browseSlice'

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

  useEffect(() => {
    dispatch(
      browseRequested({ part: 'overview', key: `${owner}/${name}`, owner, name }),
    )
  }, [dispatch, owner, name])

  const tabClass = ({ isActive }: { isActive: boolean }) =>
    isActive ? 'active' : undefined

  return (
    <div>
      <h1>
        {owner}/{name}
      </h1>
      {loading.overview && <p>Loading repository…</p>}
      {error.overview && <p role="alert">{error.overview}</p>}
      {overview?.description && <p>{overview.description}</p>}
      {overview && <code>{overview.clone_url}</code>}

      <nav>
        <NavLink end to="" className={tabClass}>
          Code
        </NavLink>{' '}
        <NavLink to="commits" className={tabClass}>
          Commits
        </NavLink>{' '}
        <NavLink to="branches" className={tabClass}>
          Branches
        </NavLink>
      </nav>

      <Outlet context={{ owner, name } satisfies RepoParams} />
    </div>
  )
}
