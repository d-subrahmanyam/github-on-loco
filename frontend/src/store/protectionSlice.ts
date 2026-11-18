import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { BranchProtectionResponse } from '../bindings/BranchProtectionResponse'
import type { PrReviewResponse } from '../bindings/PrReviewResponse'

export type ProtectionPart = 'list' | 'set' | 'remove' | 'reviews' | 'review'

export interface ProtectionRequest {
  part: ProtectionPart
  /** Request identity; late responses for an old key are discarded. */
  key: string
  owner: string
  name: string
  number?: number
  branch?: string
  params?: unknown
}

interface ProtectionState {
  keys: Record<ProtectionPart, string>
  loading: Record<ProtectionPart, boolean>
  error: Record<ProtectionPart, string | null>
  rules: BranchProtectionResponse[]
  reviews: PrReviewResponse[]
}

const PARTS: ProtectionPart[] = ['list', 'set', 'remove', 'reviews', 'review']
const emptyRecord = <T>(value: T): Record<ProtectionPart, T> =>
  Object.fromEntries(PARTS.map((part) => [part, value])) as Record<ProtectionPart, T>

const initialState: ProtectionState = {
  keys: emptyRecord(''),
  loading: emptyRecord(false),
  error: emptyRecord(null),
  rules: [],
  reviews: [],
}

const protectionSlice = createSlice({
  name: 'protection',
  initialState,
  reducers: {
    protectionRequested(state, action: PayloadAction<ProtectionRequest>) {
      const { part, key } = action.payload
      state.keys[part] = key
      state.loading[part] = true
      state.error[part] = null
    },
    protectionSucceeded(
      state,
      action: PayloadAction<{ part: ProtectionPart; key: string; data: unknown }>,
    ) {
      const { part, key, data } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      switch (part) {
        case 'list':
          state.rules = data as BranchProtectionResponse[]
          break
        case 'reviews':
          state.reviews = data as PrReviewResponse[]
          break
        case 'set': {
          const rule = data as BranchProtectionResponse
          const index = state.rules.findIndex((item) => item.branch === rule.branch)
          if (index >= 0) state.rules[index] = rule
          else state.rules.push(rule)
          break
        }
        case 'remove': {
          const branch = (data as { branch: string }).branch
          state.rules = state.rules.filter((item) => item.branch !== branch)
          break
        }
        case 'review': {
          const review = data as PrReviewResponse
          const index = state.reviews.findIndex((item) => item.author === review.author)
          if (index >= 0) state.reviews[index] = review
          else state.reviews.push(review)
          break
        }
      }
    },
    protectionFailed(
      state,
      action: PayloadAction<{ part: ProtectionPart; key: string; error: string }>,
    ) {
      const { part, key, error } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      state.error[part] = error
    },
    protectionReset() {
      return initialState
    },
  },
})

export const {
  protectionRequested,
  protectionSucceeded,
  protectionFailed,
  protectionReset,
} = protectionSlice.actions

export default protectionSlice.reducer
