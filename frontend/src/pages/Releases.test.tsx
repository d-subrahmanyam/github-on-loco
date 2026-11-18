import { render, screen, waitFor } from '@testing-library/react'
import { createMemoryRouter, Outlet, RouterProvider } from 'react-router'
import { Provider } from 'react-redux'
import { describe, expect, it, vi } from 'vitest'
import { createAppStore } from '../store'
import { jsonResponse } from '../test/helpers'
import { Releases } from './Releases'

describe('Releases', () => {
  it('lists releases with their tag and source archive', async () => {
    const release = {
      id: 1,
      tag_name: 'v1.0.0',
      title: 'Version 1.0.0',
      body: 'first stable',
      author: 'alice',
      is_prerelease: false,
      published_at: '2026-01-01T00:00:00Z',
      commit_sha: 'abcdef1234567890',
    }
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      if (String(input).endsWith('/tags')) return jsonResponse([])
      return jsonResponse([release])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    const router = createMemoryRouter(
      [
        {
          path: '/repos/:owner/:name',
          element: <Outlet context={{ owner: 'alice', name: 'hello' }} />,
          children: [{ index: true, element: <Releases /> }],
        },
      ],
      { initialEntries: ['/repos/alice/hello'] },
    )
    render(
      <Provider store={store}>
        <RouterProvider router={router} />
      </Provider>,
    )

    await waitFor(() => expect(screen.getByText('Version 1.0.0')).toBeInTheDocument())
    expect(screen.getByText(/v1\.0\.0/)).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /download source/i })).toHaveAttribute(
      'href',
      '/api/repos/alice/hello/archive/v1.0.0.tar.gz',
    )
  })
})
