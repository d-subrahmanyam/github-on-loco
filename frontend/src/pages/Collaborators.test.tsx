import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, Outlet, RouterProvider } from 'react-router'
import { Provider } from 'react-redux'
import { describe, expect, it, vi } from 'vitest'
import { browseRequested, browseSucceeded } from '../store/browseSlice'
import { createAppStore } from '../store'
import { jsonResponse } from '../test/helpers'
import { Collaborators } from './Collaborators'

function renderCollaborators() {
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
        is_private: true,
        clone_url: '',
        viewer_role: 'admin',
      },
    }),
  )
  const router = createMemoryRouter(
    [
      {
        path: '/repos/:owner/:name',
        element: <Outlet context={{ owner: 'alice', name: 'hello' }} />,
        children: [{ index: true, element: <Collaborators /> }],
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

describe('Collaborators', () => {
  it('lists grants and lets an admin add one', async () => {
    const grant = { id: 1, user_id: 2, username: 'guest', role: 'write' }
    const grants = [grant]
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') return jsonResponse(grant)
      return jsonResponse(grants)
    })
    vi.stubGlobal('fetch', fetchMock)

    renderCollaborators()

    await waitFor(() => expect(screen.getByText('guest')).toBeInTheDocument())
    expect(screen.getByLabelText('Role for guest')).toHaveValue('write')

    const user = userEvent.setup()
    await user.type(screen.getByLabelText('Username'), 'another')
    await user.click(screen.getByRole('button', { name: /Add collaborator/i }))
    expect(fetchMock.mock.calls.some((call) => call[1]?.method === 'POST')).toBe(true)
  })
})
