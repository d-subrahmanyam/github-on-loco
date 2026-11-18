import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { NotificationResponse } from '../bindings/NotificationResponse'
import type { NotificationSettingsResponse } from '../bindings/NotificationSettingsResponse'

export type NotificationPart = 'list' | 'settings'

export interface NotificationRequest {
  part: NotificationPart
  key: string
}

interface NotificationsState {
  keys: Record<NotificationPart, string>
  loading: Record<NotificationPart, boolean>
  error: Record<NotificationPart, string | null>
  items: NotificationResponse[]
  settings: NotificationSettingsResponse | null
}

const PARTS: NotificationPart[] = ['list', 'settings']
const emptyRecord = <T>(value: T): Record<NotificationPart, T> =>
  Object.fromEntries(PARTS.map((part) => [part, value])) as Record<NotificationPart, T>

const initialState: NotificationsState = {
  keys: emptyRecord(''),
  loading: emptyRecord(false),
  error: emptyRecord(null),
  items: [],
  settings: null,
}

const notificationsSlice = createSlice({
  name: 'notifications',
  initialState,
  reducers: {
    notificationRequested(state, action: PayloadAction<NotificationRequest>) {
      const { part, key } = action.payload
      state.keys[part] = key
      state.loading[part] = true
      state.error[part] = null
    },
    notificationSucceeded(
      state,
      action: PayloadAction<{ part: NotificationPart; key: string; data: unknown }>,
    ) {
      const { part, key, data } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      if (part === 'list') state.items = data as NotificationResponse[]
      else state.settings = data as NotificationSettingsResponse
    },
    notificationFailed(
      state,
      action: PayloadAction<{ part: NotificationPart; key: string; error: string }>,
    ) {
      const { part, key, error } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      state.error[part] = error
    },
    notificationsReset() {
      return initialState
    },
  },
})

export const {
  notificationRequested,
  notificationSucceeded,
  notificationFailed,
  notificationsReset,
} = notificationsSlice.actions

export default notificationsSlice.reducer
