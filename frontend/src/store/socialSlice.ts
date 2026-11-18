import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { EventResponse } from '../bindings/EventResponse'
import type { RepoStatsResponse } from '../bindings/RepoStatsResponse'

export type SocialPart = 'star' | 'unstar' | 'watch' | 'unwatch' | 'events'

export interface SocialRequest {
  part: SocialPart
  key: string
  owner: string
  name: string
}

interface SocialState {
  keys: Record<SocialPart, string>
  loading: Record<SocialPart, boolean>
  error: Record<SocialPart, string | null>
  /** Latest counts, set by a toggle; falls back to the overview on first load. */
  stats: RepoStatsResponse | null
  events: EventResponse[]
}

const PARTS: SocialPart[] = ['star', 'unstar', 'watch', 'unwatch', 'events']
const emptyRecord = <T>(value: T): Record<SocialPart, T> =>
  Object.fromEntries(PARTS.map((part) => [part, value])) as Record<SocialPart, T>

const initialState: SocialState = {
  keys: emptyRecord(''),
  loading: emptyRecord(false),
  error: emptyRecord(null),
  stats: null,
  events: [],
}

const socialSlice = createSlice({
  name: 'social',
  initialState,
  reducers: {
    socialRequested(state, action: PayloadAction<SocialRequest>) {
      const { part, key } = action.payload
      state.keys[part] = key
      state.loading[part] = true
      state.error[part] = null
    },
    socialSucceeded(
      state,
      action: PayloadAction<{ part: SocialPart; key: string; data: unknown }>,
    ) {
      const { part, key, data } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      if (part === 'events') state.events = data as EventResponse[]
      else state.stats = data as RepoStatsResponse
    },
    socialFailed(
      state,
      action: PayloadAction<{ part: SocialPart; key: string; error: string }>,
    ) {
      const { part, key, error } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      state.error[part] = error
    },
    socialReset() {
      return initialState
    },
  },
})

export const { socialRequested, socialSucceeded, socialFailed, socialReset } =
  socialSlice.actions

export default socialSlice.reducer
