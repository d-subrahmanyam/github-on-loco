import { del, get, post } from './client'
import type { CreateTokenParams } from '../bindings/CreateTokenParams'
import type { NewTokenResponse } from '../bindings/NewTokenResponse'
import type { TokenResponse } from '../bindings/TokenResponse'

export const tokensApi = {
  list: () => get<TokenResponse[]>('/api/tokens'),
  create: (params: CreateTokenParams) => post<NewTokenResponse>('/api/tokens', params),
  revoke: (id: number) => del(`/api/tokens/${id}`),
}
