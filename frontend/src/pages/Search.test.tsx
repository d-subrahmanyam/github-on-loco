import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { Provider } from 'react-redux'
import { describe, expect, it, vi } from 'vitest'
import { createAppStore } from '../store'
import { jsonResponse } from '../test/helpers'
import { Search } from './Search'

describe('Search', () => {
  it('searches and links to matching repositories', async () => {
    const user = userEvent.setup()
    const result = {
      repositories: [{
        id: 1,
        owner: 'alice',
        name: 'widget-lib',
        description: 'a widget library',
        is_private: false,
      }],
      issues: [],
      pulls: [],
      total: 1,
    }
    const fetchMock = vi.fn(async () => jsonResponse(result))
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    const router = createMemoryRouter([{ path: '/', element: <Search /> }], {
      initialEntries: ['/'],
    })
    render(
      <Provider store={store}>
        <RouterProvider router={router} />
      </Provider>,
    )

    await user.type(screen.getByLabelText('Search'), 'widget')
    await user.click(screen.getByRole('button', { name: /search/i }))

    await waitFor(() => expect(screen.getByText('alice/widget-lib')).toBeInTheDocument())
    expect(screen.getByRole('link', { name: /alice\/widget-lib/ })).toHaveAttribute(
      'href',
      '/repos/alice/widget-lib',
    )
  })
})
