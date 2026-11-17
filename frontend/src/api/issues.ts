import { get, post } from './client'
import type { AddCommentParams } from '../bindings/AddCommentParams'
import type { CommentResponse } from '../bindings/CommentResponse'
import type { CreateIssueParams } from '../bindings/CreateIssueParams'
import type { CreateLabelParams } from '../bindings/CreateLabelParams'
import type { IssueResponse } from '../bindings/IssueResponse'
import type { LabelResponse } from '../bindings/LabelResponse'

function repoPath(owner: string, name: string): string {
  return `/api/repos/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`
}

function withState(base: string, state?: string): string {
  return state ? `${base}?state=${encodeURIComponent(state)}` : base
}

export const issuesApi = {
  list: (owner: string, name: string, state?: string) =>
    get<IssueResponse[]>(withState(`${repoPath(owner, name)}/issues`, state)),

  create: (owner: string, name: string, params: CreateIssueParams) =>
    post<IssueResponse>(`${repoPath(owner, name)}/issues`, params),

  show: (owner: string, name: string, number: number) =>
    get<IssueResponse>(`${repoPath(owner, name)}/issues/${number}`),

  close: (owner: string, name: string, number: number) =>
    post<IssueResponse>(`${repoPath(owner, name)}/issues/${number}/close`, undefined),

  reopen: (owner: string, name: string, number: number) =>
    post<IssueResponse>(`${repoPath(owner, name)}/issues/${number}/reopen`, undefined),

  comments: (owner: string, name: string, number: number) =>
    get<CommentResponse[]>(`${repoPath(owner, name)}/issues/${number}/comments`),

  comment: (owner: string, name: string, number: number, params: AddCommentParams) =>
    post<CommentResponse>(`${repoPath(owner, name)}/issues/${number}/comments`, params),

  labels: (owner: string, name: string) =>
    get<LabelResponse[]>(`${repoPath(owner, name)}/labels`),

  createLabel: (owner: string, name: string, params: CreateLabelParams) =>
    post<LabelResponse>(`${repoPath(owner, name)}/labels`, params),
}
