import { get } from './client'
import type { SearchResult } from '../bindings/SearchResult'

export const searchApi = {
  run: (query: string) =>
    get<SearchResult>(`/api/search?q=${encodeURIComponent(query)}`),
}
