import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { SearchResult } from '../bindings/SearchResult'

export interface SearchRequest {
  key: string
  query: string
}

interface SearchState {
  key: string
  loading: boolean
  error: string | null
  query: string
  result: SearchResult | null
}

const initialState: SearchState = {
  key: '',
  loading: false,
  error: null,
  query: '',
  result: null,
}

const searchSlice = createSlice({
  name: 'search',
  initialState,
  reducers: {
    searchRequested(state, action: PayloadAction<SearchRequest>) {
      state.key = action.payload.key
      state.query = action.payload.query
      state.loading = true
      state.error = null
    },
    searchSucceeded(
      state,
      action: PayloadAction<{ key: string; result: SearchResult }>,
    ) {
      if (state.key !== action.payload.key) return // a newer query won
      state.loading = false
      state.result = action.payload.result
    },
    searchFailed(state, action: PayloadAction<{ key: string; error: string }>) {
      if (state.key !== action.payload.key) return
      state.loading = false
      state.error = action.payload.error
    },
    searchReset() {
      return initialState
    },
  },
})

export const { searchRequested, searchSucceeded, searchFailed, searchReset } =
  searchSlice.actions

export default searchSlice.reducer
