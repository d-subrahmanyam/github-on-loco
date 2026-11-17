const TOKEN_KEY = "auth_token";

const listeners = new Set<() => void>();

/** Subscribe to token changes so React components re-render after login/logout. */
export function subscribeToken(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Stable snapshot for `useSyncExternalStore` — localStorage strings are stable. */
export function getTokenSnapshot(): string | null {
  return window.localStorage.getItem(TOKEN_KEY);
}

function notify(): void {
  listeners.forEach((listener) => listener());
}

export function getToken(): string | null {
  return window.localStorage.getItem(TOKEN_KEY);
}

export function setToken(t: string): void {
  window.localStorage.setItem(TOKEN_KEY, t);
  notify();
}

export function clearToken(): void {
  window.localStorage.removeItem(TOKEN_KEY);
  notify();
}
