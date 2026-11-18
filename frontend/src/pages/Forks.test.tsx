import { render, screen, waitFor } from '@testing-library/react'
import { createMemoryRouter, Outlet, RouterProvider } from 'react-router'
import { Provider } from 'react-redux'
import { describe, expect, it, vi } from 'vitest'
import { createAppStore } from '../store'
import { jsonResponse } from '../test/helpers'
import { Forks } from './Forks'

describe('Forks', () => {
  it('lists forks with links to their repositories', async () => {
    const fork = {
      id: 1,
      repository_id: 2,
      owner: 'guest',
      name: 'hello',
      created_at: '2026-01-01T00:00:00Z',
    }
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      expect(String(input)).toContain('/forks')
      return jsonResponse([fork])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    const router = createMemoryRouter(
      [
        {
          path: '/repos/:owner/:name',
          element: <Outlet context={{ owner: 'alice', name: 'hello' }} />,
          children: [{ index: true, element: <Forks /> }],
        },
      ],
      { initialEntries: ['/repos/alice/hello'] },
    )
    render(
      <Provider store={store}>
        <RouterProvider router={router} />
      </Provider>,
    )

    await waitFor(() => expect(screen.getByText('guest/hello')).toBeInTheDocument())
    expect(screen.getByRole('link', { name: /guest\/hello/ })).toHaveAttribute(
      'href',
      '/repos/guest/hello',
    )
  })
})
