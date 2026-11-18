import { get, post } from './client'
import type { CreateForkParams } from '../bindings/CreateForkParams'
import type { ForkResponse } from '../bindings/ForkResponse'
import type { RepoResponse } from '../bindings/RepoResponse'

function repoPath(owner: string, name: string): string {
  return `/api/repos/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`
}

export const forksApi = {
  create: (owner: string, name: string, params: CreateForkParams) =>
    post<RepoResponse>(`${repoPath(owner, name)}/fork`, params),

  list: (owner: string, name: string) =>
    get<ForkResponse[]>(`${repoPath(owner, name)}/forks`),
}
