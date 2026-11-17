import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link } from 'react-router'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import {
  clearCreateError,
  createRequested,
  listRequested,
} from '../store/repositoriesSlice'

/** Dashboard: list the current user's repositories and create new ones. */
export function Repositories() {
  const dispatch = useAppDispatch()
  const { items, loading, error, creating, createError } = useAppSelector(
    (state) => state.repositories,
  )
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [isPrivate, setIsPrivate] = useState(false)

  useEffect(() => {
    dispatch(listRequested())
  }, [dispatch])

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    dispatch(
      createRequested({ name, description: description || null, is_private: isPrivate }),
    )
    setName('')
    setDescription('')
    setIsPrivate(false)
  }

  return (
    <div>
      <h1>Your repositories</h1>

      <form onSubmit={handleSubmit}>
        <h2>New repository</h2>
        <div>
          <label htmlFor="repo-name">Name</label>
          <input
            id="repo-name"
            required
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="hello-world"
          />
        </div>
        <div>
          <label htmlFor="repo-description">Description</label>
          <input
            id="repo-description"
            value={description}
            onChange={(event) => setDescription(event.target.value)}
          />
        </div>
        <div>
          <label htmlFor="repo-private">
            <input
              id="repo-private"
              type="checkbox"
              checked={isPrivate}
              onChange={(event) => setIsPrivate(event.target.checked)}
            />
            Private
          </label>
        </div>
        <button type="submit" disabled={creating}>
          {creating ? 'Creating…' : 'Create repository'}
        </button>
        {createError && (
          <p role="alert" onClick={() => dispatch(clearCreateError())}>
            {createError}
          </p>
        )}
      </form>

      {loading && <p>Loading repositories…</p>}
      {error && <p role="alert">{error}</p>}
      {!loading && items.length === 0 && (
        <p>No repositories yet. Create your first one above.</p>
      )}

      <ul>
        {items.map((repo) => (
          <li key={repo.id}>
            <Link to={`/repos/${repo.owner}/${repo.name}`}>
              {repo.owner}/{repo.name}
            </Link>
            {repo.description && <span> — {repo.description}</span>}
            {repo.is_private && <small> (private)</small>}
            <div>
              <code>{repo.clone_url}</code>
            </div>
          </li>
        ))}
      </ul>
    </div>
  )
}
