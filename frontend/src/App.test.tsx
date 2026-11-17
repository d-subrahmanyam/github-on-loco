import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { Provider } from 'react-redux'
import { describe, expect, it } from 'vitest'
import { App } from './App'
import { createAppStore } from './store'

function renderApp() {
  const store = createAppStore()
  const router = createMemoryRouter(
    [
      {
        path: '/',
        element: <App />,
        children: [
          { index: true, element: <div>Home page</div> },
          { path: 'login', element: <div>Login page</div> },
        ],
      },
    ],
    { initialEntries: ['/'] },
  )
  render(
    <Provider store={store}>
      <RouterProvider router={router} />
    </Provider>,
  )
  return { router }
}

describe('App header', () => {
  it('shows logout when a token exists and returns to login on click', async () => {
    window.localStorage.setItem('auth_token', 'jwt-token')
    const user = userEvent.setup()
    const { router } = renderApp()

    const logout = screen.getByRole('button', { name: /Log out/i })
    expect(logout).toBeInTheDocument()
    await user.click(logout)

    await waitFor(() => expect(window.localStorage.getItem('auth_token')).toBeNull())
    await waitFor(() => expect(router.state.location.pathname).toBe('/login'))
    expect(screen.queryByRole('button', { name: /Log out/i })).not.toBeInTheDocument()
  })

  it('hides logout for anonymous visitors', () => {
    renderApp()
    expect(screen.queryByRole('button', { name: /Log out/i })).not.toBeInTheDocument()
  })
})
