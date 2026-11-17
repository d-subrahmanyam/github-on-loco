import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { TokenResponse } from '../bindings/TokenResponse'

export type TokenPart = 'list' | 'create' | 'revoke'

export interface TokenRequest {
  part: TokenPart
  key: string
  params?: unknown
  id?: number
}

interface TokensState {
  keys: Record<TokenPart, string>
  loading: Record<TokenPart, boolean>
  error: Record<TokenPart, string | null>
  tokens: TokenResponse[]
  /** Shown once after creation, then kept until the user dismisses it. */
  plaintext: string | null
}

const PARTS: TokenPart[] = ['list', 'create', 'revoke']
const emptyRecord = <T>(value: T): Record<TokenPart, T> =>
  Object.fromEntries(PARTS.map((part) => [part, value])) as Record<TokenPart, T>

const initialState: TokensState = {
  keys: emptyRecord(''),
  loading: emptyRecord(false),
  error: emptyRecord(null),
  tokens: [],
  plaintext: null,
}

const tokensSlice = createSlice({
  name: 'tokens',
  initialState,
  reducers: {
    tokenRequested(state, action: PayloadAction<TokenRequest>) {
      const { part, key } = action.payload
      state.keys[part] = key
      state.loading[part] = true
      state.error[part] = null
    },
    tokenSucceeded(
      state,
      action: PayloadAction<{ part: TokenPart; key: string; data: unknown }>,
    ) {
      const { part, key, data } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      if (part === 'list') state.tokens = data as TokenResponse[]
      if (part === 'create') {
        const created = data as { token: string; info: TokenResponse }
        state.plaintext = created.token
        state.tokens.push(created.info)
      }
    },
    tokenFailed(
      state,
      action: PayloadAction<{ part: TokenPart; key: string; error: string }>,
    ) {
      const { part, key, error } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      state.error[part] = error
    },
    dismissPlaintext(state) {
      state.plaintext = null
    },
    tokensReset() {
      return initialState
    },
  },
})

export const {
  tokenRequested,
  tokenSucceeded,
  tokenFailed,
  dismissPlaintext,
  tokensReset,
} = tokensSlice.actions

export default tokensSlice.reducer
