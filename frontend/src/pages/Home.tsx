import { FaCodeBranch, FaGitAlt, FaGithub, FaSignInAlt, FaUserPlus } from 'react-icons/fa'
import { useSyncExternalStore } from 'react'
import { Link } from 'react-router'
import { getTokenSnapshot, subscribeToken } from '../auth/token'
import { card, primaryButton, secondaryButton } from '../ui'

export function Home() {
  const isAuthenticated = useSyncExternalStore(subscribeToken, getTokenSnapshot) !== null

  return (
    <div className="space-y-8">
      <section className={`${card} overflow-hidden`}>
        <div className="border-b border-slate-200 bg-gradient-to-r from-slate-900 to-slate-700 px-8 py-10 text-white">
          <div className="flex items-center gap-3">
            <FaGithub className="text-4xl" />
            <h1 className="text-3xl font-bold tracking-tight">github-on-loco</h1>
          </div>
          <p className="mt-3 max-w-2xl text-slate-300">
            Code hosting built with Loco: create repositories, clone and push
            real git objects over HTTP, and browse commits and files — all from
            one Rust backend and a React frontend.
          </p>
        </div>
        <div className="grid gap-4 px-8 py-6 sm:grid-cols-3">
          <div className="flex items-start gap-3">
            <FaGitAlt className="mt-1 text-2xl text-blue-600" />
            <div>
              <h2 className="font-semibold">Real git</h2>
              <p className="text-sm text-slate-600">
                clone, fetch and push via <code>git http-backend</code>
              </p>
            </div>
          </div>
          <div className="flex items-start gap-3">
            <FaGithub className="mt-1 text-2xl text-blue-600" />
            <div>
              <h2 className="font-semibold">Repositories</h2>
              <p className="text-sm text-slate-600">
                public and private metadata with username-based URLs
              </p>
            </div>
          </div>
          <div className="flex items-start gap-3">
            <FaCodeBranch className="mt-1 text-2xl text-blue-600" />
            <div>
              <h2 className="font-semibold">Browsing</h2>
              <p className="text-sm text-slate-600">
                branches, commit history, trees and raw files
              </p>
            </div>
          </div>
        </div>
      </section>

      <section className={`${card} flex flex-col items-start gap-4 p-8 sm:flex-row sm:items-center sm:justify-between`}>
        <div>
          <h2 className="text-xl font-semibold">Get started</h2>
          <p className="text-sm text-slate-600">
            {isAuthenticated
              ? 'Your repositories are ready.'
              : 'Create an account or log in to create your first repository.'}
          </p>
        </div>
        <div className="flex gap-3">
          {isAuthenticated ? (
            <Link to="/dashboard" className={primaryButton}>
              <FaGithub /> Open dashboard
            </Link>
          ) : (
            <>
              <Link to="/login" className={primaryButton}>
                <FaSignInAlt /> Log in
              </Link>
              <Link to="/register" className={secondaryButton}>
                <FaUserPlus /> Sign up
              </Link>
            </>
          )}
        </div>
      </section>
    </div>
  )
}
