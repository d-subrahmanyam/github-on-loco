import { del, get, post } from './client'
import type { BranchProtectionResponse } from '../bindings/BranchProtectionResponse'
import type { PrReviewResponse } from '../bindings/PrReviewResponse'
import type { SetBranchProtectionParams } from '../bindings/SetBranchProtectionParams'
import type { SetReviewParams } from '../bindings/SetReviewParams'

function repoPath(owner: string, name: string): string {
  return `/api/repos/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`
}

export const protectionApi = {
  list: (owner: string, name: string) =>
    get<BranchProtectionResponse[]>(`${repoPath(owner, name)}/branch-protection`),

  set: (owner: string, name: string, params: SetBranchProtectionParams) =>
    post<BranchProtectionResponse>(`${repoPath(owner, name)}/branch-protection`, params),

  remove: (owner: string, name: string, branch: string) =>
    del(`${repoPath(owner, name)}/branch-protection?branch=${encodeURIComponent(branch)}`),

  reviews: (owner: string, name: string, number: number) =>
    get<PrReviewResponse[]>(`${repoPath(owner, name)}/pulls/${number}/reviews`),

  review: (owner: string, name: string, number: number, params: SetReviewParams) =>
    post<PrReviewResponse>(`${repoPath(owner, name)}/pulls/${number}/reviews`, params),
}
