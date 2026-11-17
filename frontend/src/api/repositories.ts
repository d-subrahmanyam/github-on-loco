import { get, post } from './client'
import type { Branch } from '../bindings/Branch'
import type { Commit } from '../bindings/Commit'
import type { CommitDetail } from '../bindings/CommitDetail'
import type { CreateRepoParams } from '../bindings/CreateRepoParams'
import type { RepoResponse } from '../bindings/RepoResponse'
import type { TreeEntry } from '../bindings/TreeEntry'

function repoPath(owner: string, name: string): string {
  return `/api/repos/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`
}

/** Keeps `/` separators intact while escaping the rest of an in-tree path. */
function encodeTreePath(path: string): string {
  return encodeURIComponent(path).replace(/%2F/g, '/')
}

function withParams(
  base: string,
  params: Record<string, string | number | undefined>,
): string {
  const search = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== '') search.set(key, String(value))
  }
  const query = search.toString()
  return query ? `${base}?${query}` : base
}

export const repositoriesApi = {
  list: () => get<RepoResponse[]>('/api/repositories'),

  create: (params: CreateRepoParams) =>
    post<RepoResponse>('/api/repositories', params),

  overview: (owner: string, name: string) =>
    get<RepoResponse>(repoPath(owner, name)),

  branches: (owner: string, name: string) =>
    get<Branch[]>(`${repoPath(owner, name)}/branches`),

  commits: (owner: string, name: string, ref?: string, limit = 20) =>
    get<Commit[]>(
      withParams(`${repoPath(owner, name)}/commits`, { ref, limit }),
    ),

  commit: (owner: string, name: string, sha: string) =>
    get<CommitDetail>(`${repoPath(owner, name)}/commit/${encodeURIComponent(sha)}`),

  tree: (owner: string, name: string, ref?: string, path?: string) =>
    get<TreeEntry[]>(
      withParams(`${repoPath(owner, name)}/tree`, { ref, path }),
    ),

  /** Raw files are not JSON, so they bypass the shared JSON client. */
  async rawText(
    owner: string,
    name: string,
    ref: string | undefined,
    path: string,
  ): Promise<string> {
    const url = withParams(`${repoPath(owner, name)}/raw`, {
      ref,
      path: encodeTreePath(path),
    })
    const response = await fetch(url)
    if (!response.ok) throw new Error(`Failed to load ${path} (${response.status})`)
    return response.text()
  },
}
