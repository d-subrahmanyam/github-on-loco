import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { CreateReleaseParams } from '../bindings/CreateReleaseParams'
import type { ReleaseResponse } from '../bindings/ReleaseResponse'
import type { TagResponse } from '../bindings/TagResponse'
import type { UpdateReleaseParams } from '../bindings/UpdateReleaseParams'

export type ReleasePart =
  | 'list'
  | 'show'
  | 'create'
  | 'update'
  | 'remove'
  | 'tags'
  | 'removeTag'

export interface ReleaseRequest {
  part: ReleasePart
  /** Request identity; late responses for an old key are discarded. */
  key: string
  owner: string
  name: string
  id?: number
  tag?: string
  params?: CreateReleaseParams | UpdateReleaseParams
}

interface ReleasesState {
  keys: Record<ReleasePart, string>
  loading: Record<ReleasePart, boolean>
  error: Record<ReleasePart, string | null>
  releases: ReleaseResponse[]
  current: ReleaseResponse | null
  tags: TagResponse[]
}

const PARTS: ReleasePart[] = [
  'list', 'show', 'create', 'update', 'remove', 'tags', 'removeTag',
]
const emptyRecord = <T>(value: T): Record<ReleasePart, T> =>
  Object.fromEntries(PARTS.map((part) => [part, value])) as Record<ReleasePart, T>

const initialState: ReleasesState = {
  keys: emptyRecord(''),
  loading: emptyRecord(false),
  error: emptyRecord(null),
  releases: [],
  current: null,
  tags: [],
}

const releasesSlice = createSlice({
  name: 'releases',
  initialState,
  reducers: {
    releaseRequested(state, action: PayloadAction<ReleaseRequest>) {
      const { part, key } = action.payload
      state.keys[part] = key
      state.loading[part] = true
      state.error[part] = null
    },
    releaseSucceeded(
      state,
      action: PayloadAction<{ part: ReleasePart; key: string; data: unknown }>,
    ) {
      const { part, key, data } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      switch (part) {
        case 'list':
          state.releases = data as ReleaseResponse[]
          break
        case 'show':
          state.current = data as ReleaseResponse
          break
        case 'create': {
          const created = data as ReleaseResponse
          state.releases = [created, ...state.releases.filter((item) => item.id !== created.id)]
          state.current = created
          break
        }
        case 'update': {
          const updated = data as ReleaseResponse
          state.releases = state.releases.map((item) =>
            item.id === updated.id ? updated : item,
          )
          state.current = updated
          break
        }
        case 'remove': {
          const removedId = data as number
          state.releases = state.releases.filter((item) => item.id !== removedId)
          if (state.current?.id === removedId) state.current = null
          break
        }
        case 'tags':
          state.tags = data as TagResponse[]
          break
        case 'removeTag': {
          const removedTag = data as string
          state.tags = state.tags.filter((tag) => tag.name !== removedTag)
          break
        }
      }
    },
    releaseFailed(
      state,
      action: PayloadAction<{ part: ReleasePart; key: string; error: string }>,
    ) {
      const { part, key, error } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      state.error[part] = error
    },
    releasesReset() {
      return initialState
    },
  },
})

export const { releaseRequested, releaseSucceeded, releaseFailed, releasesReset } =
  releasesSlice.actions

export default releasesSlice.reducer
