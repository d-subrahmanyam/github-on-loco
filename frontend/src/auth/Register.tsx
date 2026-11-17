import { FaEye, FaEyeSlash, FaGithub, FaUserPlus } from 'react-icons/fa'
import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link } from 'react-router'
import { ApiClientError, post } from '../api/client'
import { card, input, label, primaryButton } from '../ui'

export function Register() {
  const [name, setName] = useState('')
  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [showPassword, setShowPassword] = useState(false)
  const [isPending, setIsPending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [isDone, setIsDone] = useState(false)

  async function handleSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    setError(null)
    setIsPending(true)
    try {
      // `register` answers 200 with a null body whether or not the address was
      // already taken — deliberately, so the endpoint cannot be used to probe
      // for accounts. There is nothing to read back, and the success copy has
      // to stay neutral for the same reason.
      await post<null>('/api/auth/register', {
        name,
        username: username || undefined,
        email,
        password,
      })
      setIsDone(true)
    } catch (err) {
      setError(err instanceof ApiClientError ? err.message : 'Failed to register')
    } finally {
      setIsPending(false)
    }
  }

  if (isDone) {
    return (
      <div className="mx-auto max-w-md">
        <div className={`${card} p-8 text-center`}>
          <FaGithub className="mx-auto text-4xl text-slate-900" />
          <h1 className="mt-4 text-2xl font-semibold">Check your email</h1>
          <p className="mt-2 text-sm text-slate-600">
            If that address is available, a verification link is on its way. You
            need to verify before you can log in.
          </p>
          <Link to="/login" className={`${primaryButton} mt-6`}>
            Back to log in
          </Link>
        </div>
      </div>
    )
  }

  return (
    <div className="mx-auto max-w-md">
      <div className={`${card} p-8`}>
        <div className="mb-6 flex items-center gap-3">
          <FaGithub className="text-3xl text-slate-900" />
          <h1 className="text-2xl font-semibold">Sign up</h1>
        </div>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="name" className={label}>
              Name
            </label>
            <input
              id="name"
              name="name"
              type="text"
              autoComplete="name"
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              className={`${input} mt-1`}
            />
          </div>
          <div>
            <label htmlFor="username" className={label}>
              Username <span className="font-normal text-slate-400">(optional)</span>
            </label>
            <input
              id="username"
              name="username"
              type="text"
              autoComplete="username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="jane-doe"
              className={`${input} mt-1`}
            />
          </div>
          <div>
            <label htmlFor="email" className={label}>
              Email
            </label>
            <input
              id="email"
              name="email"
              type="email"
              autoComplete="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className={`${input} mt-1`}
            />
          </div>
          <div>
            <label htmlFor="password" className={label}>
              Password
            </label>
            <div className="relative mt-1">
              <input
                id="password"
                name="password"
                type={showPassword ? 'text' : 'password'}
                autoComplete="new-password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className={`${input} pr-11`}
              />
              <button
                type="button"
                onClick={() => setShowPassword((visible) => !visible)}
                aria-label={showPassword ? 'Hide password' : 'Show password'}
                title={showPassword ? 'Hide password' : 'Show password'}
                className="absolute inset-y-0 right-0 flex w-11 items-center justify-center text-slate-500 hover:text-slate-800"
              >
                {showPassword ? <FaEyeSlash /> : <FaEye />}
              </button>
            </div>
          </div>
          <button type="submit" disabled={isPending} className={`${primaryButton} w-full`}>
            <FaUserPlus />
            {isPending ? 'Signing up…' : 'Sign up'}
          </button>
          {error && (
            <p role="alert" className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">
              {error}
            </p>
          )}
        </form>
        <p className="mt-6 text-center text-sm text-slate-600">
          Already have an account? <Link to="/login" className="text-blue-600 hover:underline">Log in</Link>
        </p>
      </div>
    </div>
  )
}
