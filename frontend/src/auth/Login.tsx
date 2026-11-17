import { FaEye, FaEyeSlash, FaGithub, FaSignInAlt } from 'react-icons/fa'
import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'
import { ApiClientError, post } from '../api/client'
import { card, input, label, primaryButton } from '../ui'
import { setToken } from './token'

interface LoginResponse {
  token: string
  pid: string
  name: string
  is_verified: boolean
}

export function Login() {
  const navigate = useNavigate()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [showPassword, setShowPassword] = useState(false)
  const [isPending, setIsPending] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function handleSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    setError(null)
    setIsPending(true)
    try {
      const res = await post<LoginResponse>('/api/auth/login', { email, password })
      setToken(res.token)
      navigate('/')
    } catch (err) {
      setError(err instanceof ApiClientError ? err.message : 'Failed to log in')
    } finally {
      setIsPending(false)
    }
  }

  return (
    <div className="mx-auto max-w-md">
      <div className={`${card} p-8`}>
        <div className="mb-6 flex items-center gap-3">
          <FaGithub className="text-3xl text-slate-900" />
          <h1 className="text-2xl font-semibold">Log in</h1>
        </div>
        <form onSubmit={handleSubmit} className="space-y-4">
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
                autoComplete="current-password"
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
            <FaSignInAlt />
            {isPending ? 'Logging in…' : 'Log in'}
          </button>
          {error && (
            <p role="alert" className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700">
              {error}
            </p>
          )}
        </form>
        <p className="mt-6 text-center text-sm text-slate-600">
          Don&apos;t have an account? <Link to="/register" className="text-blue-600 hover:underline">Sign up</Link>.
          You will need to verify your email before you can log in.
        </p>
      </div>
    </div>
  )
}
