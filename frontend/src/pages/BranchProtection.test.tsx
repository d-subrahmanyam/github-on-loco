import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, Outlet, RouterProvider } from 'react-router'
import { Provider } from 'react-redux'
import { describe, expect, it, vi } from 'vitest'
import { createAppStore } from '../store'
import { jsonResponse } from '../test/helpers'
import { BranchProtection } from './BranchProtection'

function renderPage() {
  const store = createAppStore()
  const router = createMemoryRouter(
    [
      {
        path: '/repos/:owner/:name',
        element: <Outlet context={{ owner: 'alice', name: 'hello' }} />,
        children: [{ index: true, element: <BranchProtection /> }],
      },
    ],
    { initialEntries: ['/repos/alice/hello'] },
  )
  render(
    <Provider store={store}>
      <RouterProvider router={router} />
    </Provider>,
  )
  return store
}

describe('BranchProtection', () => {
  it('lists rules and saves a new one', async () => {
    const user = userEvent.setup()
    const rule = {
      id: 1,
      branch: 'main',
      require_pull_request: true,
      required_approvals: 2,
    }
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') return jsonResponse(rule)
      return jsonResponse([rule])
    })
    vi.stubGlobal('fetch', fetchMock)

    renderPage()

    await waitFor(() => expect(screen.getByText('main')).toBeInTheDocument())
    expect(screen.getByText(/2 approvals required/)).toBeInTheDocument()

    await user.type(screen.getByLabelText('Branch name'), 'release')
    await user.click(screen.getByRole('button', { name: /save rule/i }))

    await waitFor(() =>
      expect(fetchMock.mock.calls.some((call) => call[1]?.method === 'POST')).toBe(true),
    )
  })
})
