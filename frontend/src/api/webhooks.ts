import { del, get, patch, post } from './client'
import type { CreateWebhookParams } from '../bindings/CreateWebhookParams'
import type { UpdateWebhookParams } from '../bindings/UpdateWebhookParams'
import type { WebhookDeliveryResponse } from '../bindings/WebhookDeliveryResponse'
import type { WebhookResponse } from '../bindings/WebhookResponse'

function repoPath(owner: string, name: string): string {
  return `/api/repos/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`
}

export const webhooksApi = {
  eventKinds: () => get<string[]>('/api/webhook-events'),

  list: (owner: string, name: string) =>
    get<WebhookResponse[]>(`${repoPath(owner, name)}/webhooks`),

  create: (owner: string, name: string, params: CreateWebhookParams) =>
    post<WebhookResponse>(`${repoPath(owner, name)}/webhooks`, params),

  update: (owner: string, name: string, id: number, params: UpdateWebhookParams) =>
    patch<WebhookResponse>(`${repoPath(owner, name)}/webhooks/${id}`, params),

  rotate: (owner: string, name: string, id: number) =>
    post<{ secret: string }>(`${repoPath(owner, name)}/webhooks/${id}/rotate`, {}),

  remove: (owner: string, name: string, id: number) =>
    del(`${repoPath(owner, name)}/webhooks/${id}`),

  deliveries: (owner: string, name: string, id: number) =>
    get<WebhookDeliveryResponse[]>(`${repoPath(owner, name)}/webhooks/${id}/deliveries`),

  retry: (owner: string, name: string, id: number, deliveryId: number) =>
    post<void>(
      `${repoPath(owner, name)}/webhooks/${id}/deliveries/${deliveryId}/retry`,
      {},
    ),
}
