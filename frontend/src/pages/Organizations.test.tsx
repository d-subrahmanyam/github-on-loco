import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { Provider } from 'react-redux'
import { describe, expect, it, vi } from 'vitest'
import { createAppStore } from '../store'
import { jsonResponse } from '../test/helpers'
import { Organizations } from './Organizations'

describe('Organizations', () => {
  it('lists organizations and creates a new one', async () => {
    const user = userEvent.setup()
    const org = {
      id: 1,
      handle: 'acme',
      display_name: 'Acme Inc.',
      description: null,
      viewer_role: 'owner',
    }
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') return jsonResponse(org)
      return jsonResponse([org])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    const router = createMemoryRouter([{ path: '/', element: <Organizations /> }], {
      initialEntries: ['/'],
    })
    render(
      <Provider store={store}>
        <RouterProvider router={router} />
      </Provider>,
    )

    await waitFor(() => expect(screen.getByText('Acme Inc.')).toBeInTheDocument())
    expect(screen.getByText(/@acme/)).toBeInTheDocument()

    await user.type(screen.getByLabelText('Handle'), 'globex')
    await user.type(screen.getByLabelText('Display name'), 'Globex')
    await user.click(screen.getByRole('button', { name: /create organization/i }))

    await waitFor(() =>
      expect(fetchMock.mock.calls.some((call) => call[1]?.method === 'POST')).toBe(true),
    )
  })
})
