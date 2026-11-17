import { describe, expect, it, vi } from 'vitest'
import { createAppStore } from './index'
import { createRequested, listRequested } from './repositoriesSlice'
import { issueRequested } from './issuesSlice'
import { tokenRequested } from './tokensSlice'
import { jsonResponse } from '../test/helpers'

describe('repository sagas', () => {
  it('loads repositories through the API into the store', async () => {
    const repos = [
      {
        id: 1,
        owner: 'alice',
        name: 'hello',
        description: null,
        is_private: false,
        clone_url: 'http://localhost:5150/alice/hello.git',
      },
    ]
    const fetchMock = vi.fn(async () => jsonResponse(repos))
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(listRequested())

    await vi.waitFor(() => expect(store.getState().repositories.items).toEqual(repos))
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/repositories',
      expect.objectContaining({ method: 'GET' }),
    )
  })

  it('creates a repo, then refreshes the list', async () => {
    const repo = {
      id: 2,
      owner: 'alice',
      name: 'new-repo',
      description: null,
      is_private: true,
      clone_url: 'http://localhost:5150/alice/new-repo.git',
    }
    const posts: unknown[] = []
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') {
        posts.push(JSON.parse(String(init.body)))
        return jsonResponse(repo, 201)
      }
      return jsonResponse([repo])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(
      createRequested({ name: 'new-repo', description: null, is_private: true }),
    )

    await vi.waitFor(() => expect(store.getState().repositories.items).toEqual([repo]))
    expect(posts).toEqual([{ name: 'new-repo', description: null, is_private: true }])
    expect(fetchMock.mock.calls.some((call) => call[1]?.method === 'GET')).toBe(true)
  })
})

describe('issue sagas', () => {
  it('ignores a stale list response that resolves after a newer one', async () => {
    let resolveFirst: ((value: Response) => void) | undefined
    const first = new Promise<Response>((resolve) => {
      resolveFirst = resolve
    })
    const oldIssues = [{ number: 1, title: 'old' }]
    const newIssues = [{ number: 2, title: 'new' }]

    const fetchMock = vi.fn(
      async (_input: RequestInfo | URL, _init?: RequestInit): Promise<Response> =>
        jsonResponse(newIssues),
    )
    fetchMock.mockImplementationOnce(async () => first)
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(issueRequested({ part: 'list', key: 'first', owner: 'a', name: 'r' }))
    store.dispatch(issueRequested({ part: 'list', key: 'second', owner: 'a', name: 'r' }))

    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2))
    await vi.waitFor(() => expect(store.getState().issues.issues).toEqual(newIssues))

    resolveFirst?.(jsonResponse(oldIssues))
    await new Promise((resolve) => setTimeout(resolve, 0))

    expect(store.getState().issues.issues).toEqual(newIssues)
  })
})

describe('token sagas', () => {
  it('issues a token and refreshes the list', async () => {
    const token = {
      id: 1,
      name: 'laptop',
      scopes: null,
      expires_at: null,
      last_used_at: null,
      created_at: '2026-01-01T00:00:00Z',
    }
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') {
        return jsonResponse({ token: 'glp_1_secret', info: token })
      }
      return jsonResponse([token])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(
      tokenRequested({ part: 'create', key: 'create', params: { name: 'laptop' } }),
    )

    await vi.waitFor(() => expect(store.getState().tokens.tokens).toEqual([token]))
    expect(store.getState().tokens.plaintext).toBe('glp_1_secret')
  })
})
