import { render, screen, waitFor } from '@testing-library/react'
import { createMemoryRouter, Outlet, RouterProvider } from 'react-router'
import { Provider } from 'react-redux'
import { describe, expect, it, vi } from 'vitest'
import { browseRequested } from '../store/browseSlice'
import { createAppStore } from '../store'
import { jsonResponse } from '../test/helpers'
import { Webhooks } from './Webhooks'

describe('Webhooks', () => {
  it('lists webhooks for an admin and hides the secret', async () => {
    const hook = {
      id: 1,
      url: 'https://example.com/hook',
      events: 'star',
      is_active: true,
      created_by: 'alice',
      secret: '••••••••',
    }
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url.endsWith('/webhook-events')) return jsonResponse(['star', 'push'])
      if (url.endsWith('/repos/alice/hello')) return jsonResponse({ viewer_role: 'admin' })
      return jsonResponse([hook])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(browseRequested({ part: 'overview', key: 'ov', owner: 'alice', name: 'hello' }))
    const router = createMemoryRouter(
      [
        {
          path: '/repos/:owner/:name',
          element: <Outlet context={{ owner: 'alice', name: 'hello' }} />,
          children: [{ index: true, element: <Webhooks /> }],
        },
      ],
      { initialEntries: ['/repos/alice/hello'] },
    )
    render(
      <Provider store={store}>
        <RouterProvider router={router} />
      </Provider>,
    )

    await waitFor(() => expect(screen.getByText(hook.url)).toBeInTheDocument())
    expect(screen.getByText(/by alice/)).toBeInTheDocument()
    expect(screen.queryByText('whsec_abc')).toBeNull()
  })
})
