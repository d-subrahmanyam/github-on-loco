import { del, get, post, request } from './client'
import type { AddCollaboratorParams } from '../bindings/AddCollaboratorParams'
import type { CollaboratorResponse } from '../bindings/CollaboratorResponse'
import type { UpdateCollaboratorParams } from '../bindings/UpdateCollaboratorParams'

function repoPath(owner: string, name: string): string {
  return `/api/repos/${encodeURIComponent(owner)}/${encodeURIComponent(name)}/collaborators`
}

export const collaboratorsApi = {
  list: (owner: string, name: string) =>
    get<CollaboratorResponse[]>(repoPath(owner, name)),

  add: (owner: string, name: string, params: AddCollaboratorParams) =>
    post<CollaboratorResponse>(repoPath(owner, name), params),

  update: (owner: string, name: string, userId: number, params: UpdateCollaboratorParams) =>
    request<CollaboratorResponse>('PATCH', `${repoPath(owner, name)}/${userId}`, params),

  remove: (owner: string, name: string, userId: number) =>
    del(`${repoPath(owner, name)}/${userId}`),
}
