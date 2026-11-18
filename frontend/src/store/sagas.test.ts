import { describe, expect, it, vi } from 'vitest'
import { collaboratorRequested } from './collaboratorsSlice'
import { createAppStore } from './index'
import { createRequested, listRequested } from './repositoriesSlice'
import { issueRequested } from './issuesSlice'
import { protectionRequested } from './protectionSlice'
import { pullRequested } from './pullsSlice'
import { socialRequested } from './socialSlice'
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
        clone_url: 'http://localhost:5150/alice/hello.git', viewer_role: 'owner', star_count: 0, watch_count: 0, starred: false, watching: false,
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
      clone_url: 'http://localhost:5150/alice/new-repo.git', viewer_role: 'owner', star_count: 0, watch_count: 0, starred: false, watching: false,
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

describe('pull request sagas', () => {
  const merged = {
    id: 1,
    number: 1,
    title: 'Add feature',
    body: null,
    state: 'merged',
    author: 'alice',
    head_ref: 'feature',
    head_sha: 'abc123',
    base_ref: 'main',
    comment_count: 0,
    required_approvals: 0,
    approval_count: 0,
    merge_blocked_reason: null,
    merged_at: '2026-01-01T00:00:00Z',
    merged_commit_sha: 'def456',
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
  }

  it('merges a PR, then refreshes the list, detail and comments', async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input)
      if (init?.method === 'POST' && url.endsWith('/merge')) return jsonResponse(merged)
      if (url.endsWith('/comments')) return jsonResponse([])
      if (url.includes('/pulls/1')) return jsonResponse(merged)
      return jsonResponse([merged])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(pullRequested({ part: 'merge', key: 'merge', owner: 'a', name: 'r', number: 1 }))

    await vi.waitFor(() => expect(store.getState().pulls.current?.state).toBe('merged'))
    expect(store.getState().pulls.pulls).toEqual([merged])
    expect(fetchMock.mock.calls.some((call) => call[1]?.method === 'POST')).toBe(true)
  })

  it('keeps a compare result separate from the detail response', async () => {
    const comparison = {
      base: 'base-sha',
      head: 'abc123',
      ahead_by: 1,
      behind_by: 0,
      commits: [],
      files: [],
    }
    const fetchMock = vi.fn(async () => jsonResponse(comparison))
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(
      pullRequested({
        part: 'compare',
        key: 'compare',
        owner: 'a',
        name: 'r',
        head: 'abc123',
        base: 'main',
      }),
    )

    await vi.waitFor(() => expect(store.getState().pulls.compare).toEqual(comparison))
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('head=abc123'),
      expect.anything(),
    )
  })
})

describe('collaborator sagas', () => {
  it('adds a collaborator and stores the returned grant', async () => {
    const grant = { id: 1, user_id: 2, username: 'guest', role: 'write' }
    const fetchMock = vi.fn(async () => jsonResponse(grant))
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(
      collaboratorRequested({
        part: 'add',
        key: 'add',
        owner: 'a',
        name: 'r',
        username: 'guest',
        role: 'write',
      }),
    )

    await vi.waitFor(() =>
      expect(store.getState().collaborators.collaborators).toEqual([grant]),
    )
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/collaborators'),
      expect.objectContaining({ method: 'POST' }),
    )
  })
})

describe('protection sagas', () => {
  it('saves a rule and refreshes the rule list', async () => {
    const rule = {
      id: 1,
      branch: 'main',
      require_pull_request: true,
      required_approvals: 2,
    }
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') return jsonResponse(rule)
      return jsonResponse([rule])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(
      protectionRequested({
        part: 'set',
        key: 'set',
        owner: 'a',
        name: 'r',
        params: {
          branch: 'main',
          require_pull_request: true,
          required_approvals: 2,
        },
      }),
    )

    await vi.waitFor(() => expect(store.getState().protection.rules).toEqual([rule]))
    expect(fetchMock.mock.calls.some((call) => call[1]?.method === 'POST')).toBe(true)
  })

  it('reloads reviews and the PR after an approval', async () => {
    const review = {
      id: 1,
      author: 'reviewer',
      state: 'approved',
      body: null,
      created_at: '2026-01-01T00:00:00Z',
    }
    const pr = {
      id: 1,
      number: 1,
      title: 'A change',
      body: null,
      state: 'open',
      author: 'author',
      head_ref: 'feature',
      head_sha: 'abc',
      base_ref: 'main',
      comment_count: 0,
      required_approvals: 1,
      approval_count: 1,
      merge_blocked_reason: null,
      merged_at: null,
      merged_commit_sha: null,
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
    }
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') return jsonResponse(review)
      if (String(_input).includes('/reviews')) return jsonResponse([review])
      return jsonResponse(pr)
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(
      protectionRequested({
        part: 'review',
        key: 'review',
        owner: 'a',
        name: 'r',
        number: 1,
        params: { state: 'approved', body: null },
      }),
    )

    await vi.waitFor(() => expect(store.getState().protection.reviews).toEqual([review]))
    expect(store.getState().pulls.current?.approval_count).toBe(1)
  })
})

describe('social sagas', () => {
  it('stars a repo and refreshes the activity feed', async () => {
    const stats = { star_count: 1, watch_count: 0, starred: true, watching: false }
    const events = [
      {
        id: 1,
        actor: 'guest',
        kind: 'star',
        text: 'starred this repository',
        created_at: '2026-01-01T00:00:00Z',
      },
    ]
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') return jsonResponse(stats)
      return jsonResponse(events)
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(socialRequested({ part: 'star', key: 'star', owner: 'a', name: 'r' }))

    await vi.waitFor(() => expect(store.getState().social.stats).toEqual(stats))
    expect(store.getState().social.events).toEqual(events)
    expect(fetchMock.mock.calls.some((call) => call[1]?.method === 'POST')).toBe(true)
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
