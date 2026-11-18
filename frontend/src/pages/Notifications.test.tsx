import { render, screen, waitFor } from '@testing-library/react'
import { Provider } from 'react-redux'
import { describe, expect, it, vi } from 'vitest'
import { createAppStore } from '../store'
import { jsonResponse } from '../test/helpers'
import { Notifications } from './Notifications'

describe('Notifications', () => {
  it('shows the flag state and delivery history', async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      if (String(input).endsWith('/settings')) {
        return jsonResponse({ enabled: false, channels: ['email'], available: ['email', 'log'] })
      }
      return jsonResponse([{
        id: 1,
        event: 'star',
        title: 'New activity in alice/hello',
        body: 'bob triggered `star`.',
        channel: 'log',
        status: 'sent',
        attempts: 1,
        created_at: '2026-01-01T00:00:00Z',
        delivered_at: '2026-01-01T00:00:01Z',
      }])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    render(
      <Provider store={store}>
        <Notifications />
      </Provider>,
    )

    await waitFor(() => expect(screen.getByText('disabled')).toBeInTheDocument())
    expect(screen.getByText('New activity in alice/hello')).toBeInTheDocument()
    expect(screen.getByText(/Available implementations/)).toBeInTheDocument()
  })
})
