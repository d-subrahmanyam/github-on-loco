import { createBrowserRouter } from 'react-router'
import { App } from './App'
import { Login } from './auth/Login'
import { Register } from './auth/Register'
import { RequireAuth } from './auth/RequireAuth'
import { BranchProtection } from './pages/BranchProtection'
import { Branches } from './pages/Branches'
import { Collaborators } from './pages/Collaborators'
import { CommitDetail } from './pages/CommitDetail'
import { Forks } from './pages/Forks'
import { Commits } from './pages/Commits'
import { Home } from './pages/Home'
import { IssueDetail } from './pages/IssueDetail'
import { IssueList } from './pages/IssueList'
import { PullDetail } from './pages/PullDetail'
import { PullList } from './pages/PullList'
import { RepoActivity } from './pages/RepoActivity'
import { Repositories } from './pages/Repositories'
import { RepoCode } from './pages/RepoCode'
import { Tokens } from './pages/Tokens'
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
          { path: 'tokens', element: <Tokens /> },
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
          { path: 'issues', element: <IssueList /> },
          { path: 'issues/:number', element: <IssueDetail /> },
          { path: 'pulls', element: <PullList /> },
          { path: 'pulls/:number', element: <PullDetail /> },
          { path: 'forks', element: <Forks /> },
          { path: 'collaborators', element: <Collaborators /> },
          { path: 'branch-protection', element: <BranchProtection /> },
          { path: 'activity', element: <RepoActivity /> },
        ],
      },
    ],
  },
])
