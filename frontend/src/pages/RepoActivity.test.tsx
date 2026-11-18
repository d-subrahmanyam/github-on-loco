import { render, screen, waitFor } from '@testing-library/react'
import { createMemoryRouter, Outlet, RouterProvider } from 'react-router'
import { Provider } from 'react-redux'
import { describe, expect, it, vi } from 'vitest'
import { createAppStore } from '../store'
import { jsonResponse } from '../test/helpers'
import { RepoActivity } from './RepoActivity'

describe('RepoActivity', () => {
  it('renders events from the activity endpoint', async () => {
    const event = {
      id: 1,
      actor: 'alice',
      kind: 'pr.merged',
      text: 'merged pull request #1',
      created_at: '2026-01-01T00:00:00Z',
    }
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      expect(String(input)).toContain('/events')
      return jsonResponse([event])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    const router = createMemoryRouter(
      [
        {
          path: '/repos/:owner/:name',
          element: <Outlet context={{ owner: 'alice', name: 'hello' }} />,
          children: [{ index: true, element: <RepoActivity /> }],
        },
      ],
      { initialEntries: ['/repos/alice/hello'] },
    )
    render(
      <Provider store={store}>
        <RouterProvider router={router} />
      </Provider>,
    )

    await waitFor(() => expect(screen.getByText(/merged pull request #1/)).toBeInTheDocument())
    expect(screen.getByText('alice')).toBeInTheDocument()
  })
})
