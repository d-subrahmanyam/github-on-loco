import { FaCopy, FaKey, FaPlus, FaTrash } from 'react-icons/fa'
import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { dismissPlaintext, tokenRequested } from '../store/tokensSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, input, label, primaryButton } from '../ui'

/** Manage personal access tokens used by git remotes. */
export function Tokens() {
  const dispatch = useAppDispatch()
  const { tokens, plaintext, loading, error } = useAppSelector((state) => state.tokens)
  const [name, setName] = useState('')
  const [expires, setExpires] = useState('')

  useEffect(() => {
    dispatch(tokenRequested({ part: 'list', key: 'list:tokens' }))
  }, [dispatch])

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    dispatch(
      tokenRequested({
        part: 'create',
        key: `create:${Date.now()}`,
        params: {
          name,
          expires_in_days: expires ? Number(expires) : null,
        },
      }),
    )
    setName('')
    setExpires('')
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-3">
        <FaKey className="text-3xl text-slate-900" />
        <div>
          <h1 className="text-2xl font-semibold">Access tokens</h1>
          <p className="text-sm text-slate-600">
            Use a token as the password in a git remote instead of your account password.
          </p>
        </div>
      </div>

      {plaintext && (
        <div className="rounded-xl border border-green-300 bg-green-50 p-5">
          <p className="text-sm font-medium text-green-800">
            Copy this token now — it will not be shown again.
          </p>
          <div className="mt-2 flex items-center gap-2">
            <code className="flex-1 truncate rounded-lg bg-white px-3 py-2 text-sm">{plaintext}</code>
            <button
              type="button"
              className="inline-flex items-center gap-2 rounded-lg border border-green-300 bg-white px-3 py-2 text-sm text-green-800"
              onClick={() => void navigator.clipboard.writeText(plaintext)}
            >
              <FaCopy /> Copy
            </button>
            <button
              type="button"
              className="text-sm text-green-800 underline"
              onClick={() => dispatch(dismissPlaintext())}
            >
              Done
            </button>
          </div>
        </div>
      )}

      <div className="grid gap-6 lg:grid-cols-[320px_1fr]">
        <form onSubmit={handleSubmit} className={`${card} h-fit space-y-4 p-6`}>
          <h2 className="flex items-center gap-2 text-lg font-semibold">
            <FaPlus className="text-blue-600" /> New token
          </h2>
          <div>
            <label htmlFor="token-name" className={label}>Name</label>
            <input
              id="token-name"
              required
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="laptop"
              className={`${input} mt-1`}
            />
          </div>
          <div>
            <label htmlFor="token-expiry" className={label}>
              Expires in days <span className="font-normal text-slate-400">(optional)</span>
            </label>
            <input
              id="token-expiry"
              type="number"
              min={1}
              value={expires}
              onChange={(event) => setExpires(event.target.value)}
              className={`${input} mt-1`}
            />
          </div>
          <button type="submit" disabled={loading.create} className={`${primaryButton} w-full`}>
            <FaPlus /> {loading.create ? 'Creating…' : 'Create token'}
          </button>
          {error.create && <p role="alert" className="text-sm text-red-700">{error.create}</p>}
        </form>

        <section className={`${card} p-6`}>
          <h2 className="mb-4 text-lg font-semibold">Your tokens</h2>
          {loading.list && <p className="text-sm text-slate-500">Loading tokens…</p>}
          {error.list && <p role="alert" className="text-sm text-red-700">{error.list}</p>}
          {!loading.list && tokens.length === 0 && (
            <p className="text-sm text-slate-500">No tokens yet.</p>
          )}
          <ul className="divide-y divide-slate-200">
            {tokens.map((token) => (
              <li key={token.id} className="flex items-center gap-3 py-4 first:pt-0">
                <FaKey className="text-slate-400" />
                <div className="flex-1">
                  <p className="font-medium text-slate-800">{token.name}</p>
                  <p className="text-xs text-slate-500">
                    Created {new Date(token.created_at).toLocaleDateString()}
                    {token.expires_at && ` · Expires ${new Date(token.expires_at).toLocaleDateString()}`}
                    {token.last_used_at && ` · Last used ${new Date(token.last_used_at).toLocaleDateString()}`}
                  </p>
                </div>
                <button
                  type="button"
                  title="Revoke token"
                  onClick={() =>
                    dispatch(
                      tokenRequested({ part: 'revoke', key: `revoke:${token.id}`, id: token.id }),
                    )
                  }
                  className="rounded-lg p-2 text-red-600 hover:bg-red-50"
                >
                  <FaTrash />
                </button>
              </li>
            ))}
          </ul>
        </section>
      </div>
    </div>
  )
}
