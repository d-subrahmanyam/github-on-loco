import { get } from './client'
import type { NotificationResponse } from '../bindings/NotificationResponse'
import type { NotificationSettingsResponse } from '../bindings/NotificationSettingsResponse'

export const notificationsApi = {
  list: () => get<NotificationResponse[]>('/api/notifications'),
  settings: () => get<NotificationSettingsResponse>('/api/notifications/settings'),
}
