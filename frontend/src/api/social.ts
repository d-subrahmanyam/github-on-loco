import { get, request } from './client'
import type { EventResponse } from '../bindings/EventResponse'
import type { RepoStatsResponse } from '../bindings/RepoStatsResponse'

function repoPath(owner: string, name: string): string {
  return `/api/repos/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`
}

export const socialApi = {
  star: (owner: string, name: string) =>
    request<RepoStatsResponse>('POST', `${repoPath(owner, name)}/star`),

  unstar: (owner: string, name: string) =>
    request<RepoStatsResponse>('DELETE', `${repoPath(owner, name)}/star`),

  watch: (owner: string, name: string) =>
    request<RepoStatsResponse>('POST', `${repoPath(owner, name)}/watch`),

  unwatch: (owner: string, name: string) =>
    request<RepoStatsResponse>('DELETE', `${repoPath(owner, name)}/watch`),

  events: (owner: string, name: string, limit = 20) =>
    get<EventResponse[]>(`${repoPath(owner, name)}/events?limit=${limit}`),
}
