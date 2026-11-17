import { createBrowserRouter } from 'react-router'
import { App } from './App'
import { Login } from './auth/Login'
import { Register } from './auth/Register'
import { RequireAuth } from './auth/RequireAuth'
import { Branches } from './pages/Branches'
import { CommitDetail } from './pages/CommitDetail'
import { Commits } from './pages/Commits'
import { Home } from './pages/Home'
import { Repositories } from './pages/Repositories'
import { RepoCode } from './pages/RepoCode'
import { RepoLayout } from './pages/RepoLayout'
// scaffold:imports

export const router = createBrowserRouter([
  {
    path: '/',
    element: <App />,
    children: [
      { index: true, element: <Home /> },
      { path: 'login', element: <Login /> },
      { path: 'register', element: <Register /> },
      {
        element: <RequireAuth />,
        children: [
          // scaffold:routes
          { path: 'dashboard', element: <Repositories /> },
        ],
      },
      {
        path: 'repos/:owner/:name',
        element: <RepoLayout />,
        children: [
          { index: true, element: <RepoCode /> },
          { path: 'commits', element: <Commits /> },
          { path: 'commit/:sha', element: <CommitDetail /> },
          { path: 'branches', element: <Branches /> },
        ],
      },
    ],
  },
])
