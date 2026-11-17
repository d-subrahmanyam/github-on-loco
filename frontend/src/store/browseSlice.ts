import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { Branch } from '../bindings/Branch'
import type { Commit } from '../bindings/Commit'
import type { CommitDetail } from '../bindings/CommitDetail'
import type { RepoResponse } from '../bindings/RepoResponse'
import type { TreeEntry } from '../bindings/TreeEntry'

export type BrowsePart =
  | 'overview'
  | 'branches'
  | 'commits'
  | 'detail'
  | 'tree'
  | 'file'

export interface BrowseRequest {
  part: BrowsePart
  /** Identifies the params behind the request; stale responses are ignored. */
  key: string
  owner: string
  name: string
  ref?: string
  path?: string
  sha?: string
}

interface BrowseState {
  keys: Record<BrowsePart, string>
  loading: Record<BrowsePart, boolean>
  error: Record<BrowsePart, string | null>
  overview: RepoResponse | null
  branches: Branch[]
  commits: Commit[]
  detail: CommitDetail | null
  tree: TreeEntry[]
  file: string | null
}

const initialState: BrowseState = {
  keys: {
    overview: '',
    branches: '',
    commits: '',
    detail: '',
    tree: '',
    file: '',
  },
  loading: {
    overview: false,
    branches: false,
    commits: false,
    detail: false,
    tree: false,
    file: false,
  },
  error: {
    overview: null,
    branches: null,
    commits: null,
    detail: null,
    tree: null,
    file: null,
  },
  overview: null,
  branches: [],
  commits: [],
  detail: null,
  tree: [],
  file: null,
}

const browseSlice = createSlice({
  name: 'browse',
  initialState,
  reducers: {
    browseRequested(state, action: PayloadAction<BrowseRequest>) {
      const { part, key } = action.payload
      state.keys[part] = key
      state.loading[part] = true
      state.error[part] = null
    },
    browseSucceeded(
      state,
      action: PayloadAction<{ part: BrowsePart; key: string; data: unknown }>,
    ) {
      const { part, key, data } = action.payload
      if (state.keys[part] !== key) return // a newer request won
      state.loading[part] = false
      switch (part) {
        case 'overview':
          state.overview = data as RepoResponse
          break
        case 'branches':
          state.branches = data as Branch[]
          break
        case 'commits':
          state.commits = data as Commit[]
          break
        case 'detail':
          state.detail = data as CommitDetail
          break
        case 'tree':
          state.tree = data as TreeEntry[]
          break
        case 'file':
          state.file = data as string
          break
      }
    },
    browseFailed(
      state,
      action: PayloadAction<{ part: BrowsePart; key: string; error: string }>,
    ) {
      const { part, key, error } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      state.error[part] = error
    },
    browseReset() {
      // A fresh repo/branch changes every key; components simply issue new
      // requests, and stale responses are dropped by the key check above.
      return initialState
    },
  },
})

export const { browseRequested, browseSucceeded, browseFailed, browseReset } =
  browseSlice.actions

export default browseSlice.reducer
