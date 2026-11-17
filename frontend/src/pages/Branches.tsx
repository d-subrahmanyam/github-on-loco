import { useEffect } from 'react'
import { Link } from 'react-router'
import { browseRequested } from '../store/browseSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
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
    <div>
      <h2>Branches</h2>
      {loading.branches && <p>Loading branches…</p>}
      {error.branches && <p role="alert">{error.branches}</p>}
      <ul>
        {branches.map((branch) => (
          <li key={branch.name}>
            <Link to={`?ref=${encodeURIComponent(branch.name)}`}>{branch.name}</Link>{' '}
            <code>{branch.sha.slice(0, 12)}</code>
          </li>
        ))}
      </ul>
    </div>
  )
}
