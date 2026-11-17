import { get, post } from './client'
import type { AddReviewCommentParams } from '../bindings/AddReviewCommentParams'
import type { CompareResult } from '../bindings/CompareResult'
import type { CreatePullRequestParams } from '../bindings/CreatePullRequestParams'
import type { PullRequestResponse } from '../bindings/PullRequestResponse'
import type { ReviewCommentResponse } from '../bindings/ReviewCommentResponse'

function repoPath(owner: string, name: string): string {
  return `/api/repos/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`
}

function withState(base: string, state?: string): string {
  return state ? `${base}?state=${encodeURIComponent(state)}` : base
}

export const pullsApi = {
  list: (owner: string, name: string, state?: string) =>
    get<PullRequestResponse[]>(withState(`${repoPath(owner, name)}/pulls`, state)),

  show: (owner: string, name: string, number: number) =>
    get<PullRequestResponse>(`${repoPath(owner, name)}/pulls/${number}`),

  create: (owner: string, name: string, params: CreatePullRequestParams) =>
    post<PullRequestResponse>(`${repoPath(owner, name)}/pulls`, params),

  compare: (owner: string, name: string, head: string, base: string) => {
    const query = new URLSearchParams({ head })
    if (base) query.set('base', base)
    return get<CompareResult>(`${repoPath(owner, name)}/pulls/compare?${query}`)
  },

  comments: (owner: string, name: string, number: number) =>
    get<ReviewCommentResponse[]>(`${repoPath(owner, name)}/pulls/${number}/comments`),

  comment: (owner: string, name: string, number: number, params: AddReviewCommentParams) =>
    post<ReviewCommentResponse>(`${repoPath(owner, name)}/pulls/${number}/comments`, params),

  close: (owner: string, name: string, number: number) =>
    post<PullRequestResponse>(`${repoPath(owner, name)}/pulls/${number}/close`, undefined),

  reopen: (owner: string, name: string, number: number) =>
    post<PullRequestResponse>(`${repoPath(owner, name)}/pulls/${number}/reopen`, undefined),

  merge: (owner: string, name: string, number: number) =>
    post<PullRequestResponse>(`${repoPath(owner, name)}/pulls/${number}/merge`, undefined),
}
