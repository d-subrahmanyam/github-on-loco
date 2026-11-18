import { describe, expect, it, vi } from 'vitest'
import { collaboratorRequested } from './collaboratorsSlice'
import { createAppStore } from './index'
import { forkRequested } from './forksSlice'
import { createRequested, listRequested } from './repositoriesSlice'
import { issueRequested } from './issuesSlice'
import { orgRequested } from './organizationsSlice'
import { protectionRequested } from './protectionSlice'
import { pullRequested } from './pullsSlice'
import { releaseRequested } from './releasesSlice'
import { socialRequested } from './socialSlice'
import { tokenRequested } from './tokensSlice'
import { notificationRequested } from './notificationsSlice'
import { webhookRequested } from './webhooksSlice'
import { searchRequested } from './searchSlice'
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
        clone_url: 'http://localhost:5150/alice/hello.git', viewer_role: 'owner', star_count: 0, watch_count: 0, starred: false, watching: false, fork_count: 0, parent: null,
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
      clone_url: 'http://localhost:5150/alice/new-repo.git', viewer_role: 'owner', star_count: 0, watch_count: 0, starred: false, watching: false, fork_count: 0, parent: null,
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
      createRequested({ name: 'new-repo', description: null, is_private: true, organization: null }),
    )

    await vi.waitFor(() => expect(store.getState().repositories.items).toEqual([repo]))
    expect(posts).toEqual([
      { name: 'new-repo', description: null, is_private: true, organization: null },
    ])
    expect(fetchMock.mock.calls.some((call) => call[1]?.method === 'GET')).toBe(true)
  })
})

describe('release sagas', () => {
  const release = {
    id: 1,
    tag_name: 'v1.0.0',
    title: 'Version 1.0.0',
    body: 'notes',
    author: 'alice',
    is_prerelease: false,
    published_at: '2026-01-01T00:00:00Z',
    commit_sha: 'abcdef1234567890',
  }
  const tag = { name: 'v1.0.0', sha: 'abcdef1234567890', created_at: '2026-01-01T00:00:00Z' }

  it('lists releases and tags, then refreshes both after publishing', async () => {
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') return jsonResponse(release)
      const url = String(_input)
      return jsonResponse(url.endsWith('/tags') ? [tag] : [release])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    const request = { owner: 'alice', name: 'hello' }
    store.dispatch(releaseRequested({ part: 'list', key: 'releases:alice/hello', ...request }))
    store.dispatch(releaseRequested({ part: 'tags', key: 'tags:alice/hello', ...request }))
    await vi.waitFor(() => expect(store.getState().releases.releases).toEqual([release]))
    expect(store.getState().releases.tags).toEqual([tag])

    store.dispatch(
      releaseRequested({
        part: 'create',
        key: 'create:v1',
        ...request,
        params: {
          tag_name: 'v1.0.0',
          target: 'main',
          title: 'Version 1.0.0',
          body: 'notes',
          is_prerelease: false,
        },
      }),
    )
    await vi.waitFor(() => expect(store.getState().releases.current?.id).toBe(1))
    expect(fetchMock.mock.calls.some((call) => call[1]?.method === 'POST')).toBe(true)
  })
})

describe('webhook sagas', () => {
  const hook = {
    id: 1,
    url: 'https://example.com/hook',
    events: 'star',
    is_active: true,
    created_by: 'alice',
    secret: 'whsec_abc',
  }

  it('loads webhooks and kinds, then refreshes after a create', async () => {
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(_input)
      if (url.endsWith('/webhook-events')) return jsonResponse(['star', 'push'])
      if (init?.method === 'POST') return jsonResponse(hook)
      return jsonResponse([hook])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    const base = { owner: 'alice', name: 'hello' }
    store.dispatch(webhookRequested({ part: 'list', key: 'webhooks:alice/hello', ...base }))
    store.dispatch(webhookRequested({ part: 'kinds', key: 'kinds', ...base }))
    await vi.waitFor(() => expect(store.getState().webhooks.webhooks).toEqual([hook]))
    expect(store.getState().webhooks.kinds).toEqual(['star', 'push'])

    store.dispatch(
      webhookRequested({
        part: 'create',
        key: 'create-hook',
        ...base,
        params: { url: hook.url, events: 'star' },
      }),
    )
    await vi.waitFor(() => expect(store.getState().webhooks.createdSecret?.secret).toBe('whsec_abc'))
    expect(fetchMock.mock.calls.some((call) => call[1]?.method === 'POST')).toBe(true)
  })
})

describe('search sagas', () => {
  it('loads grouped results and ignores a stale response', async () => {
    const result = {
      repositories: [{
        id: 1,
        owner: 'alice',
        name: 'hello',
        description: null,
        is_private: false,
      }],
      issues: [],
      pulls: [],
      total: 1,
    }
    const fetchMock = vi.fn(async () => jsonResponse(result))
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(searchRequested({ key: 'old', query: 'old' }))
    store.dispatch(searchRequested({ key: 'new', query: 'widget' }))
    await vi.waitFor(() => expect(store.getState().search.result).toEqual(result))
    expect(store.getState().search.query).toBe('widget')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/search?q=widget',
      expect.objectContaining({ method: 'GET' }),
    )
  })
})

describe('notification sagas', () => {
  it('loads the delivery history and flag settings', async () => {
    const item = {
      id: 1,
      event: 'star',
      title: 'New activity in alice/hello',
      body: 'bob triggered `star`.',
      channel: 'log',
      status: 'sent',
      attempts: 1,
      created_at: '2026-01-01T00:00:00Z',
      delivered_at: '2026-01-01T00:00:01Z',
    }
    const settings = { enabled: false, channels: ['email'], available: ['email', 'log'] }
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      if (String(input).endsWith('/settings')) return jsonResponse(settings)
      return jsonResponse([item])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(notificationRequested({ part: 'list', key: 'list' }))
    store.dispatch(notificationRequested({ part: 'settings', key: 'settings' }))
    await vi.waitFor(() => expect(store.getState().notifications.items).toEqual([item]))
    expect(store.getState().notifications.settings).toEqual(settings)
  })
})

describe('fork sagas', () => {
  const forkRepo = {
    id: 2,
    owner: 'guest',
    name: 'hello',
    description: null,
    is_private: false,
    clone_url: 'http://localhost:5150/guest/hello.git',
    viewer_role: 'owner',
    star_count: 0,
    watch_count: 0,
    starred: false,
    watching: false,
    fork_count: 0,
    parent: { owner: 'alice', name: 'hello' },
  }

  it('creates a fork and keeps the response tagged with its request key', async () => {
    const fetchMock = vi.fn(async () => jsonResponse(forkRepo))
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(forkRequested({ part: 'create', key: 'fork-1', owner: 'alice', name: 'hello' }))

    await vi.waitFor(() => expect(store.getState().forks.created?.key).toBe('fork-1'))
    expect(store.getState().forks.created?.data).toEqual(forkRepo)
  })

  it('lists forks through the API', async () => {
    const forks = [
      {
        id: 1,
        repository_id: 2,
        owner: 'guest',
        name: 'hello',
        created_at: '2026-01-01T00:00:00Z',
      },
    ]
    const fetchMock = vi.fn(async () => jsonResponse(forks))
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(forkRequested({ part: 'list', key: 'list', owner: 'alice', name: 'hello' }))

    await vi.waitFor(() => expect(store.getState().forks.forks).toEqual(forks))
  })
})

describe('organization sagas', () => {
  const org = {
    id: 1,
    handle: 'acme',
    display_name: 'Acme Inc.',
    description: null,
    viewer_role: 'owner',
  }

  it('lists organizations and appends a newly created one', async () => {
    const fetchMock = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) => {
      if (init?.method === 'POST') return jsonResponse(org)
      return jsonResponse([org])
    })
    vi.stubGlobal('fetch', fetchMock)

    const store = createAppStore()
    store.dispatch(orgRequested({ part: 'list', key: 'list:orgs' }))
    await vi.waitFor(() => expect(store.getState().organizations.orgs).toEqual([org]))

    store.dispatch(
      orgRequested({
        part: 'create',
        key: 'create:acme',
        params: { handle: 'acme', display_name: 'Acme Inc.', description: null },
      }),
    )
    await vi.waitFor(() => expect(store.getState().organizations.current?.handle).toBe('acme'))
    expect(fetchMock.mock.calls.some((call) => call[1]?.method === 'POST')).toBe(true)
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
