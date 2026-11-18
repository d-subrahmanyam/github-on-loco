import { del, get, patch, post } from './client'
import type { CreateReleaseParams } from '../bindings/CreateReleaseParams'
import type { ReleaseResponse } from '../bindings/ReleaseResponse'
import type { TagResponse } from '../bindings/TagResponse'
import type { UpdateReleaseParams } from '../bindings/UpdateReleaseParams'

function repoPath(owner: string, name: string): string {
  return `/api/repos/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`
}

export const releasesApi = {
  list: (owner: string, name: string) =>
    get<ReleaseResponse[]>(`${repoPath(owner, name)}/releases`),

  show: (owner: string, name: string, id: number) =>
    get<ReleaseResponse>(`${repoPath(owner, name)}/releases/${id}`),

  create: (owner: string, name: string, params: CreateReleaseParams) =>
    post<ReleaseResponse>(`${repoPath(owner, name)}/releases`, params),

  update: (owner: string, name: string, id: number, params: UpdateReleaseParams) =>
    patch<ReleaseResponse>(`${repoPath(owner, name)}/releases/${id}`, params),

  remove: (owner: string, name: string, id: number) =>
    del(`${repoPath(owner, name)}/releases/${id}`),

  tags: (owner: string, name: string) =>
    get<TagResponse[]>(`${repoPath(owner, name)}/tags`),

  removeTag: (owner: string, name: string, tag: string) =>
    del(`${repoPath(owner, name)}/tags/${encodeURIComponent(tag)}`),

  archiveUrl: (owner: string, name: string, reference: string): string =>
    `${repoPath(owner, name)}/archive/${encodeURIComponent(reference)}.tar.gz`,
}
