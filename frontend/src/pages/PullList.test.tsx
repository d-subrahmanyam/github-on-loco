import { render, screen, waitFor } from '@testing-library/react'
import { createMemoryRouter, Outlet, RouterProvider } from 'react-router'
import { Provider } from 'react-redux'
import { describe, expect, it, vi } from 'vitest'
import { browseRequested, browseSucceeded } from '../store/browseSlice'
import { createAppStore } from '../store'
import { jsonResponse } from '../test/helpers'
import { PullList } from './PullList'

function renderPulls() {
  const store = createAppStore()
  store.dispatch(browseRequested({ part: 'overview', key: 'ov', owner: 'alice', name: 'hello' }))
  store.dispatch(
    browseSucceeded({
      part: 'overview',
      key: 'ov',
      data: {
        id: 1,
        owner: 'alice',
        name: 'hello',
        description: null,
        is_private: false,
        clone_url: '',
        viewer_role: 'owner',
      },
    }),
  )
  const router = createMemoryRouter(
    [
      {
        path: '/repos/:owner/:name',
        element: <Outlet context={{ owner: 'alice', name: 'hello' }} />,
        children: [
          { index: true, element: <PullList /> },
          { path: 'pulls', element: <PullList /> },
          { path: 'pulls/:number', element: <div>PR detail</div> },
        ],
      },
    ],
    { initialEntries: ['/repos/alice/hello/pulls'] },
  )
  render(
    <Provider store={store}>
      <RouterProvider router={router} />
    </Provider>,
  )
}

describe('PullList', () => {
  it('renders PRs and offers branches from the compare endpoint', async () => {
    const pr = {
      id: 1,
      number: 7,
      title: 'Add feature',
      body: 'details',
      state: 'open',
      author: 'alice',
      head_ref: 'feature',
      head_sha: 'abc123',
      base_ref: 'main',
      comment_count: 2,
      merged_at: null,
      merged_commit_sha: null,
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
    }
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const path = new URL(String(input), 'http://localhost').pathname
      if (path.endsWith('/branches')) {
        return jsonResponse([
          { name: 'main', sha: 'aaa' },
          { name: 'feature', sha: 'bbb' },
        ])
      }
      if (path.endsWith('/pulls')) return jsonResponse([pr])
      return jsonResponse([])
    })
    vi.stubGlobal('fetch', fetchMock)

    renderPulls()

    await waitFor(() => expect(screen.getByText('Add feature')).toBeInTheDocument())
    expect(screen.getByText(/#7 opened by alice · 2 comments/)).toBeInTheDocument()
    expect(screen.getAllByText('feature').length).toBeGreaterThan(0)
    await waitFor(() =>
      expect(screen.getByLabelText('Compare branch')).toHaveValue('feature'),
    )
  })
})
