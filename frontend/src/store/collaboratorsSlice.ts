import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { CollaboratorResponse } from '../bindings/CollaboratorResponse'

export type CollaboratorPart = 'list' | 'add' | 'update' | 'remove'

export interface CollaboratorRequest {
  part: CollaboratorPart
  key: string
  owner: string
  name: string
  username?: string
  role?: string
  userId?: number
}

interface CollaboratorsState {
  keys: Record<CollaboratorPart, string>
  loading: Record<CollaboratorPart, boolean>
  error: Record<CollaboratorPart, string | null>
  collaborators: CollaboratorResponse[]
}

const PARTS: CollaboratorPart[] = ['list', 'add', 'update', 'remove']
const emptyRecord = <T>(value: T): Record<CollaboratorPart, T> =>
  Object.fromEntries(PARTS.map((part) => [part, value])) as Record<CollaboratorPart, T>

const initialState: CollaboratorsState = {
  keys: emptyRecord(''),
  loading: emptyRecord(false),
  error: emptyRecord(null),
  collaborators: [],
}

const collaboratorsSlice = createSlice({
  name: 'collaborators',
  initialState,
  reducers: {
    collaboratorRequested(state, action: PayloadAction<CollaboratorRequest>) {
      const { part, key } = action.payload
      state.keys[part] = key
      state.loading[part] = true
      state.error[part] = null
    },
    collaboratorSucceeded(
      state,
      action: PayloadAction<{ part: CollaboratorPart; key: string; data: unknown }>,
    ) {
      const { part, key, data } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      if (part === 'list') {
        state.collaborators = data as CollaboratorResponse[]
      } else if (part === 'add') {
        state.collaborators.push(data as CollaboratorResponse)
        state.collaborators.sort((a, b) => a.username.localeCompare(b.username))
      } else if (part === 'update') {
        const updated = data as CollaboratorResponse
        const index = state.collaborators.findIndex((item) => item.id === updated.id)
        if (index >= 0) state.collaborators[index] = updated
      } else if (part === 'remove') {
        state.collaborators = state.collaborators.filter(
          (item) => item.user_id !== (data as { user_id: number }).user_id,
        )
      }
    },
    collaboratorFailed(
      state,
      action: PayloadAction<{ part: CollaboratorPart; key: string; error: string }>,
    ) {
      const { part, key, error } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      state.error[part] = error
    },
    collaboratorsReset() {
      return initialState
    },
  },
})

export const {
  collaboratorRequested,
  collaboratorSucceeded,
  collaboratorFailed,
  collaboratorsReset,
} = collaboratorsSlice.actions

export default collaboratorsSlice.reducer
