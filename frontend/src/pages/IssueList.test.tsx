import { render, screen, waitFor } from '@testing-library/react'
import { createMemoryRouter, Outlet, RouterProvider } from 'react-router'
import { Provider } from 'react-redux'
import { describe, expect, it, vi } from 'vitest'
import { createAppStore } from '../store'
import { jsonResponse } from '../test/helpers'
import { IssueList } from './IssueList'

function renderIssues() {
  const store = createAppStore()
  const router = createMemoryRouter(
    [
      {
        path: '/repos/:owner/:name',
        element: <Outlet context={{ owner: 'alice', name: 'hello' }} />,
        children: [{ index: true, element: <IssueList /> }],
      },
    ],
    { initialEntries: ['/repos/alice/hello'] },
  )
  render(
    <Provider store={store}>
      <RouterProvider router={router} />
    </Provider>,
  )
}

describe('IssueList', () => {
  it('renders issues and labels returned by the API', async () => {
    const issue = {
      id: 1,
      number: 7,
      title: 'Login is broken',
      body: 'details',
      state: 'open',
      author: 'alice',
      comment_count: 2,
      labels: [{ id: 3, name: 'bug', color: 'ff0000' }],
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
    }
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const path = new URL(String(input), 'http://localhost').pathname
      if (path.endsWith('/issues')) return jsonResponse([issue])
      if (path.endsWith('/labels')) {
        return jsonResponse([{ id: 3, name: 'bug', color: 'ff0000' }])
      }
      return jsonResponse([])
    })
    vi.stubGlobal('fetch', fetchMock)

    renderIssues()

    await waitFor(() => expect(screen.getByText('Login is broken')).toBeInTheDocument())
    expect(screen.getByText(/#7 opened by alice/)).toBeInTheDocument()
    expect(screen.getAllByText('bug').length).toBeGreaterThan(0)
  })
})
