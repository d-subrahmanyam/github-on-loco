import { Link } from 'react-router'
import { getToken } from '../auth/token'

export function Home() {
  return (
    <div>
      <h1>Welcome to Loco</h1>
      <nav>
        <ul>
          {/* scaffold:nav */}
        </ul>
      </nav>
      {getToken() ? (
        <p>
          <Link to="/dashboard">Go to your repositories</Link>.
        </p>
      ) : (
        <p>
          <Link to="/login">Log in</Link> or <Link to="/register">sign up</Link> to
          get started.
        </p>
      )}
    </div>
  )
}
