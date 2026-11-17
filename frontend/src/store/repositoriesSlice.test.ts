import { describe, expect, it } from 'vitest'
import reducer, {
  createFailed,
  createStarted,
  createSucceeded,
  listFailed,
  listRequested,
  listSucceeded,
  reset,
} from './repositoriesSlice'
import type { RepoResponse } from '../bindings/RepoResponse'

const repo: RepoResponse = {
  id: 1,
  owner: 'alice',
  name: 'hello',
  description: null,
  is_private: false,
  clone_url: 'http://localhost:5150/alice/hello.git', viewer_role: 'owner',
}

const initial = reducer(undefined, { type: '@@INIT' })

describe('repositoriesSlice', () => {
  it('tracks a list request through success and failure', () => {
    const loading = reducer(initial, listRequested())
    expect(loading.loading).toBe(true)
    expect(loading.error).toBeNull()

    const loaded = reducer(loading, listSucceeded([repo]))
    expect(loaded.loading).toBe(false)
    expect(loaded.items).toEqual([repo])

    const failed = reducer(loading, listFailed('offline'))
    expect(failed.loading).toBe(false)
    expect(failed.error).toBe('offline')
  })

  it('prepends a created repo and clears the form error', () => {
    const started = reducer(initial, createStarted())
    expect(started.creating).toBe(true)

    const created = reducer(started, createSucceeded(repo))
    expect(created.creating).toBe(false)
    expect(created.items).toEqual([repo])

    const failed = reducer(initial, createFailed('bad name'))
    expect(failed.createError).toBe('bad name');
    expect(reducer(failed, reset())).toEqual(initial)
  })
})
