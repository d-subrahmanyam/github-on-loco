import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { Provider } from 'react-redux'
import { describe, expect, it, vi } from 'vitest'
import { createAppStore } from '../store'
import { jsonResponse } from '../test/helpers'
import { Login } from './Login'

function renderLogin() {
  const store = createAppStore()
  const router = createMemoryRouter(
    [
      { path: '/login', element: <Login /> },
      { path: '/', element: <div>Home page</div> },
    ],
    { initialEntries: ['/login'] },
  )
  render(
    <Provider store={store}>
      <RouterProvider router={router} />
    </Provider>,
  )
  return { router }
}

describe('Login', () => {
  it('lets the user reveal and hide the password', async () => {
    const user = userEvent.setup()
    renderLogin()

    const password = screen.getByLabelText('Password')
    expect(password).toHaveAttribute('type', 'password')

    await user.click(screen.getByRole('button', { name: 'Show password' }))
    expect(password).toHaveAttribute('type', 'text')

    await user.click(screen.getByRole('button', { name: 'Hide password' }))
    expect(password).toHaveAttribute('type', 'password')
  })

  it('stores the token and navigates home on a successful login', async () => {
    const user = userEvent.setup()
    const fetchMock = vi.fn(async () =>
      jsonResponse({
        token: 'jwt-token',
        pid: 'pid-1',
        name: 'Alice',
        is_verified: true,
      }),
    )
    vi.stubGlobal('fetch', fetchMock)

    const { router } = renderLogin()
    await user.type(screen.getByLabelText('Email'), 'alice@example.com')
    await user.type(screen.getByLabelText('Password'), 'secret')
    await user.click(screen.getByRole('button', { name: /Log in/i }))

    await waitFor(() => expect(window.localStorage.getItem('auth_token')).toBe('jwt-token'))
    await waitFor(() => expect(router.state.location.pathname).toBe('/'))
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/auth/login',
      expect.objectContaining({ method: 'POST' }),
    )
  })
})
