import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { CreateWebhookParams } from '../bindings/CreateWebhookParams'
import type { UpdateWebhookParams } from '../bindings/UpdateWebhookParams'
import type { WebhookDeliveryResponse } from '../bindings/WebhookDeliveryResponse'
import type { WebhookResponse } from '../bindings/WebhookResponse'

export type WebhookPart =
  | 'list'
  | 'kinds'
  | 'create'
  | 'update'
  | 'remove'
  | 'rotate'
  | 'deliveries'
  | 'retry'

export interface WebhookRequest {
  part: WebhookPart
  key: string
  owner: string
  name: string
  id?: number
  deliveryId?: number
  params?: CreateWebhookParams | UpdateWebhookParams
}

interface WebhooksState {
  keys: Record<WebhookPart, string>
  loading: Record<WebhookPart, boolean>
  error: Record<WebhookPart, string | null>
  webhooks: WebhookResponse[]
  kinds: string[]
  deliveries: Record<number, WebhookDeliveryResponse[]>
  createdSecret: { key: string; secret: string } | null
}

const PARTS: WebhookPart[] = [
  'list', 'kinds', 'create', 'update', 'remove', 'rotate', 'deliveries', 'retry',
]
const emptyRecord = <T>(value: T): Record<WebhookPart, T> =>
  Object.fromEntries(PARTS.map((part) => [part, value])) as Record<WebhookPart, T>

const initialState: WebhooksState = {
  keys: emptyRecord(''),
  loading: emptyRecord(false),
  error: emptyRecord(null),
  webhooks: [],
  kinds: [],
  deliveries: {},
  createdSecret: null,
}

const webhooksSlice = createSlice({
  name: 'webhooks',
  initialState,
  reducers: {
    webhookRequested(state, action: PayloadAction<WebhookRequest>) {
      const { part, key } = action.payload
      state.keys[part] = key
      state.loading[part] = true
      state.error[part] = null
      if (part === 'create') state.createdSecret = null
    },
    webhookSucceeded(
      state,
      action: PayloadAction<{ part: WebhookPart; key: string; data: unknown }>,
    ) {
      const { part, key, data } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      switch (part) {
        case 'list':
          state.webhooks = data as WebhookResponse[]
          break
        case 'kinds':
          state.kinds = data as string[]
          break
        case 'create': {
          const created = data as WebhookResponse
          state.webhooks.push(created)
          state.createdSecret = { key, secret: created.secret }
          break
        }
        case 'update': {
          const updated = data as WebhookResponse
          state.webhooks = state.webhooks.map((item) =>
            item.id === updated.id ? updated : item,
          )
          break
        }
        case 'remove': {
          const removedId = data as number
          state.webhooks = state.webhooks.filter((item) => item.id !== removedId)
          delete state.deliveries[removedId]
          break
        }
        case 'rotate': {
          const result = data as { id: number; secret: string }
          state.webhooks = state.webhooks.map((item) =>
            item.id === result.id ? { ...item, secret: result.secret } : item,
          )
          state.createdSecret = { key, secret: result.secret }
          break
        }
        case 'deliveries': {
          const result = data as { id: number; items: WebhookDeliveryResponse[] }
          state.deliveries[result.id] = result.items
          break
        }
        case 'retry':
          break
      }
    },
    webhookFailed(
      state,
      action: PayloadAction<{ part: WebhookPart; key: string; error: string }>,
    ) {
      const { part, key, error } = action.payload
      if (state.keys[part] !== key) return
      state.loading[part] = false
      state.error[part] = error
    },
    webhooksReset() {
      return initialState
    },
  },
})

export const { webhookRequested, webhookSucceeded, webhookFailed, webhooksReset } =
  webhooksSlice.actions

export default webhooksSlice.reducer
