import { FaCode, FaExclamationCircle, FaGitAlt, FaSearch } from 'react-icons/fa'
import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link } from 'react-router'
import { searchRequested } from '../store/searchSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, input, primaryButton } from '../ui'

/** Global, permission-aware search over repositories, issues and pull requests. */
export function Search() {
  const dispatch = useAppDispatch()
  const { result, loading, error, query } = useAppSelector((state) => state.search)
  const [term, setTerm] = useState(query)

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const trimmed = term.trim()
    if (trimmed.length < 2) return
    dispatch(
      searchRequested({ key: `${trimmed}:${Date.now()}`, query: trimmed }),
    )
  }

  const empty =
    result &&
    result.repositories.length === 0 &&
    result.issues.length === 0 &&
    result.pulls.length === 0

  return (
    <div className="mx-auto max-w-4xl space-y-6">
      <form onSubmit={handleSubmit} className={`${card} flex gap-2 p-4`}>
        <input
          aria-label="Search"
          value={term}
          onChange={(event) => setTerm(event.target.value)}
          placeholder="Search repositories, issues and pull requests"
          className={input}
        />
        <button type="submit" disabled={loading || term.trim().length < 2} className={primaryButton}>
          <FaSearch /> Search
        </button>
      </form>

      {error && <p role="alert" className="text-sm text-red-700">{error}</p>}
      {loading && <p className="text-sm text-slate-500">Searching…</p>}
      {empty && !loading && (
        <p className="text-sm text-slate-500">No matches for “{query || term}”.</p>
      )}

      {result && result.repositories.length > 0 && (
        <section className={`${card} overflow-hidden`}>
          <h2 className="border-b border-slate-200 bg-slate-50 px-5 py-3 font-semibold">
            Repositories ({result.repositories.length})
          </h2>
          <ul className="divide-y divide-slate-100">
            {result.repositories.map((repo) => (
              <li key={repo.id} className="px-5 py-3">
                <Link
                  to={`/repos/${repo.owner}/${repo.name}`}
                  className="flex items-center gap-2 font-medium text-blue-600 hover:underline"
                >
                  <FaCode /> {repo.owner}/{repo.name}
                </Link>
                {repo.description && (
                  <p className="mt-1 text-sm text-slate-600">{repo.description}</p>
                )}
              </li>
            ))}
          </ul>
        </section>
      )}

      {result && result.issues.length > 0 && (
        <section className={`${card} overflow-hidden`}>
          <h2 className="border-b border-slate-200 bg-slate-50 px-5 py-3 font-semibold">
            Issues ({result.issues.length})
          </h2>
          <ul className="divide-y divide-slate-100">
            {result.issues.map((issue) => (
              <li key={issue.id} className="px-5 py-3">
                <Link
                  to={`/repos/${issue.owner}/${issue.repo}/issues/${issue.number}`}
                  className="flex items-center gap-2 text-blue-600 hover:underline"
                >
                  <FaExclamationCircle /> {issue.title}
                </Link>
                <p className="mt-1 text-xs text-slate-500">
                  {issue.owner}/{issue.repo} #{issue.number} · {issue.state}
                </p>
              </li>
            ))}
          </ul>
        </section>
      )}

      {result && result.pulls.length > 0 && (
        <section className={`${card} overflow-hidden`}>
          <h2 className="border-b border-slate-200 bg-slate-50 px-5 py-3 font-semibold">
            Pull requests ({result.pulls.length})
          </h2>
          <ul className="divide-y divide-slate-100">
            {result.pulls.map((pull) => (
              <li key={pull.id} className="px-5 py-3">
                <Link
                  to={`/repos/${pull.owner}/${pull.repo}/pulls/${pull.number}`}
                  className="flex items-center gap-2 text-blue-600 hover:underline"
                >
                  <FaGitAlt /> {pull.title}
                </Link>
                <p className="mt-1 text-xs text-slate-500">
                  {pull.owner}/{pull.repo} #{pull.number} · {pull.state}
                </p>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  )
}
