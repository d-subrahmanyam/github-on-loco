import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { CommentResponse } from '../bindings/CommentResponse'
import type { IssueResponse } from '../bindings/IssueResponse'
import type { LabelResponse } from '../bindings/LabelResponse'

export type IssuePart =
  | 'list'
  | 'detail'
  | 'comments'
  | 'labels'
  | 'create'
  | 'comment'
  | 'state'
  | 'createLabel'

export interface IssueRequest {
  part: IssuePart
  /** Request identity; late responses for an old key are discarded. */
  key: string
  owner: string
  name: string
  number?: number
  state?: string
  listState?: string
  params?: unknown
}

interface IssuesState {
  keys: Record<IssuePart, string>
  loading: Record<IssuePart, boolean>
  error: Record<IssuePart, string | null>
  issues: IssueResponse[]
  current: IssueResponse | null
  comments: CommentResponse[]
  labels: LabelResponse[]
}

const PARTS: IssuePart[] = [
  'list',
  'detail',
  'comments',
  'labels',
  'create',
  'comment',
  'state',
  'createLabel',
]

function emptyRecord<T>(value: T): Record<IssuePart, T> {
  return Object.fromEntries(PARTS.map((part) => [part, value])) as Record<IssuePart, T>
}

const initialState: IssuesState = {
  keys: emptyRecord(''),
  loading: emptyRecord(false),
  error: emptyRecord(null),
  issues: [],
  current: null,
  comments: [],
  labels: [],
}

const issuesSlice = createSlice({
  name: 'issues',
  initialState,
  reducers: {
    issueRequested(state, action: PayloadAction<IssueRequest>) {
      const { part, key } = action.payload
      state.keys[part] = key
      state.loading[part] = true
      state.error[part] = null
    },
    issueSucceeded(
      state,
      action: PayloadAction<{ part: IssuePart; key: string; data: unknown }>,
    ) {
      const { part, key, data } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      switch (part) {
        case 'list':
          state.issues = data as IssueResponse[]
          break
        case 'detail':
        case 'create':
        case 'state':
          state.current = data as IssueResponse
          break
        case 'comments':
          state.comments = data as CommentResponse[]
          break
        case 'labels':
          state.labels = data as LabelResponse[]
          break
        case 'createLabel': {
          const label = data as LabelResponse
          state.labels.push(label)
          state.labels.sort((a, b) => a.name.localeCompare(b.name))
          break
        }
        case 'comment':
          state.comments.push(data as CommentResponse)
          break
      }
    },
    issueFailed(
      state,
      action: PayloadAction<{ part: IssuePart; key: string; error: string }>,
    ) {
      const { part, key, error } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      state.error[part] = error
    },
    issuesReset() {
      return initialState
    },
  },
})

export const { issueRequested, issueSucceeded, issueFailed, issuesReset } =
  issuesSlice.actions

export default issuesSlice.reducer
