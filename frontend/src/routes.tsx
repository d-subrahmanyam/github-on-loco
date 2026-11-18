import { createBrowserRouter } from 'react-router'
import { App } from './App'
import { Login } from './auth/Login'
import { OrganizationDetail } from './pages/OrganizationDetail'
import { Register } from './auth/Register'
import { RequireAuth } from './auth/RequireAuth'
import { BranchProtection } from './pages/BranchProtection'
import { Branches } from './pages/Branches'
import { Collaborators } from './pages/Collaborators'
import { CommitDetail } from './pages/CommitDetail'
import { Forks } from './pages/Forks'
import { Commits } from './pages/Commits'
import { Home } from './pages/Home'
import { Organizations } from './pages/Organizations'
import { IssueDetail } from './pages/IssueDetail'
import { IssueList } from './pages/IssueList'
import { PullDetail } from './pages/PullDetail'
import { PullList } from './pages/PullList'
import { RepoActivity } from './pages/RepoActivity'
import { Repositories } from './pages/Repositories'
import { Releases } from './pages/Releases'
import { RepoCode } from './pages/RepoCode'
import { Tokens } from './pages/Tokens'
import { Webhooks } from './pages/Webhooks'
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
          { path: 'organizations', element: <Organizations /> },
          { path: 'organizations/:handle', element: <OrganizationDetail /> },
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
          { path: 'releases', element: <Releases /> },
          { path: 'collaborators', element: <Collaborators /> },
          { path: 'branch-protection', element: <BranchProtection /> },
          { path: 'webhooks', element: <Webhooks /> },
          { path: 'activity', element: <RepoActivity /> },
        ],
      },
    ],
  },
])
