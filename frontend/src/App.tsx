import { FaSignOutAlt } from 'react-icons/fa'
import { useSyncExternalStore } from 'react'
import { Link, Outlet, useNavigate } from 'react-router'
import { clearToken, getTokenSnapshot, subscribeToken } from './auth/token'
import { browseReset } from './store/browseSlice'
import { useAppDispatch } from './store/hooks'
import { reset as resetRepositories } from './store/repositoriesSlice'
import { button } from './ui'

export function App() {
  const navigate = useNavigate()
  const dispatch = useAppDispatch()
  // Reactive: `getToken()` alone would not re-render this layout after a login,
  // which is why the logout control previously never appeared.
  const token = useSyncExternalStore(subscribeToken, getTokenSnapshot)
  const isAuthenticated = token !== null

  function handleLogout() {
    clearToken()
    dispatch(browseReset())
    dispatch(resetRepositories())
    navigate('/login', { replace: true })
  }

  return (
    <div className="min-h-screen">
      <header className="border-b border-slate-800 bg-slate-900 text-slate-100">
        <div className="mx-auto flex max-w-6xl items-center gap-6 px-4 py-3">
          <Link to="/" className="flex items-center gap-2 text-lg font-semibold">
            <img
              src="/github-loco-logo.png"
              alt="github-on-loco logo"
              className="h-9 w-auto"
            />
            <span>github-on-loco</span>
          </Link>
          <nav className="flex flex-1 items-center gap-5 text-sm">
            <Link to="/" className="text-slate-300 hover:text-white">
              Home
            </Link>
            {isAuthenticated && (
              <Link to="/dashboard" className="text-slate-300 hover:text-white">
                Repositories
              </Link>
            )}
          </nav>
          {isAuthenticated && (
            <button
              type="button"
              onClick={handleLogout}
              className={`${button} border border-slate-700 text-slate-200 hover:bg-slate-800`}
            >
              <FaSignOutAlt /> Log out
            </button>
          )}
        </div>
      </header>

      <main className="mx-auto max-w-6xl px-4 py-8">
        <Outlet />
      </main>
    </div>
  )
}
