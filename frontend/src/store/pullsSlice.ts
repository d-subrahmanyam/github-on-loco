import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { CompareResult } from '../bindings/CompareResult'
import type { PullRequestResponse } from '../bindings/PullRequestResponse'
import type { ReviewCommentResponse } from '../bindings/ReviewCommentResponse'

export type PullPart =
  | 'list'
  | 'detail'
  | 'compare'
  | 'comments'
  | 'create'
  | 'comment'
  | 'state'
  | 'merge'

export interface PullRequest {
  part: PullPart
  /** Request identity; late responses for an old key are discarded. */
  key: string
  owner: string
  name: string
  number?: number
  state?: string
  listState?: string
  head?: string
  base?: string
  params?: unknown
}

interface PullsState {
  keys: Record<PullPart, string>
  loading: Record<PullPart, boolean>
  error: Record<PullPart, string | null>
  pulls: PullRequestResponse[]
  current: PullRequestResponse | null
  compare: CompareResult | null
  comments: ReviewCommentResponse[]
}

const PARTS: PullPart[] = [
  'list',
  'detail',
  'compare',
  'comments',
  'create',
  'comment',
  'state',
  'merge',
]

function emptyRecord<T>(value: T): Record<PullPart, T> {
  return Object.fromEntries(PARTS.map((part) => [part, value])) as Record<PullPart, T>
}

const initialState: PullsState = {
  keys: emptyRecord(''),
  loading: emptyRecord(false),
  error: emptyRecord(null),
  pulls: [],
  current: null,
  compare: null,
  comments: [],
}

const pullsSlice = createSlice({
  name: 'pulls',
  initialState,
  reducers: {
    pullRequested(state, action: PayloadAction<PullRequest>) {
      const { part, key } = action.payload
      state.keys[part] = key
      state.loading[part] = true
      state.error[part] = null
    },
    pullSucceeded(
      state,
      action: PayloadAction<{ part: PullPart; key: string; data: unknown }>,
    ) {
      const { part, key, data } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      switch (part) {
        case 'list':
          state.pulls = data as PullRequestResponse[]
          break
        case 'detail':
        case 'create':
        case 'state':
        case 'merge':
          state.current = data as PullRequestResponse
          break
        case 'compare':
          state.compare = data as CompareResult
          break
        case 'comments':
          state.comments = data as ReviewCommentResponse[]
          break
        case 'comment':
          state.comments.push(data as ReviewCommentResponse)
          break
      }
    },
    pullFailed(
      state,
      action: PayloadAction<{ part: PullPart; key: string; error: string }>,
    ) {
      const { part, key, error } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      state.error[part] = error
    },
    pullsReset() {
      return initialState
    },
  },
})

export const { pullRequested, pullSucceeded, pullFailed, pullsReset } =
  pullsSlice.actions

export default pullsSlice.reducer
