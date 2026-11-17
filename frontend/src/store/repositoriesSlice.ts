import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { CreateRepoParams } from '../bindings/CreateRepoParams'
import type { RepoResponse } from '../bindings/RepoResponse'

interface RepositoriesState {
  items: RepoResponse[]
  loading: boolean
  error: string | null
  creating: boolean
  createError: string | null
}

const initialState: RepositoriesState = {
  items: [],
  loading: false,
  error: null,
  creating: false,
  createError: null,
}

const repositoriesSlice = createSlice({
  name: 'repositories',
  initialState,
  reducers: {
    listRequested(state) {
      state.loading = true
      state.error = null
    },
    listSucceeded(state, action: PayloadAction<RepoResponse[]>) {
      state.loading = false
      state.items = action.payload
    },
    listFailed(state, action: PayloadAction<string>) {
      state.loading = false
      state.error = action.payload
    },
    createRequested(_state, _action: PayloadAction<CreateRepoParams>) {
      // Handled by the saga; the slice only records the pending state.
    },
    createStarted(state) {
      state.creating = true
      state.createError = null
    },
    createSucceeded(state, action: PayloadAction<RepoResponse>) {
      state.creating = false
      state.items.unshift(action.payload)
    },
    createFailed(state, action: PayloadAction<string>) {
      state.creating = false
      state.createError = action.payload
    },
    clearCreateError(state) {
      state.createError = null
    },
  },
})

export const {
  listRequested,
  listSucceeded,
  listFailed,
  createRequested,
  createStarted,
  createSucceeded,
  createFailed,
  clearCreateError,
} = repositoriesSlice.actions

export default repositoriesSlice.reducer
