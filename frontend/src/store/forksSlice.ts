import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { ForkResponse } from '../bindings/ForkResponse'
import type { RepoResponse } from '../bindings/RepoResponse'

export type ForkPart = 'create' | 'list'

export interface ForkRequest {
  part: ForkPart
  /** Request identity; late responses for an old key are discarded. */
  key: string
  owner: string
  name: string
  forkName?: string
}

interface ForksState {
  keys: Record<ForkPart, string>
  loading: Record<ForkPart, boolean>
  error: Record<ForkPart, string | null>
  forks: ForkResponse[]
  /** The new repository, tagged with its request key so the UI can navigate once. */
  created: { key: string; data: RepoResponse } | null
}

const PARTS: ForkPart[] = ['create', 'list']
const emptyRecord = <T>(value: T): Record<ForkPart, T> =>
  Object.fromEntries(PARTS.map((part) => [part, value])) as Record<ForkPart, T>

const initialState: ForksState = {
  keys: emptyRecord(''),
  loading: emptyRecord(false),
  error: emptyRecord(null),
  forks: [],
  created: null,
}

const forksSlice = createSlice({
  name: 'forks',
  initialState,
  reducers: {
    forkRequested(state, action: PayloadAction<ForkRequest>) {
      const { part, key } = action.payload
      state.keys[part] = key
      state.loading[part] = true
      state.error[part] = null
      if (part === 'create') state.created = null
    },
    forkSucceeded(
      state,
      action: PayloadAction<{ part: ForkPart; key: string; data: unknown }>,
    ) {
      const { part, key, data } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      if (part === 'list') state.forks = data as ForkResponse[]
      else state.created = { key, data: data as RepoResponse }
    },
    forkFailed(
      state,
      action: PayloadAction<{ part: ForkPart; key: string; error: string }>,
    ) {
      const { part, key, error } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      state.error[part] = error
    },
    forksReset() {
      return initialState
    },
  },
})

export const { forkRequested, forkSucceeded, forkFailed, forksReset } =
  forksSlice.actions

export default forksSlice.reducer
