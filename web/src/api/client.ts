import createClient from 'openapi-fetch';
import type { paths } from './schema';

export const TOKEN_STORAGE_KEY = 'let_timer_token';
export const WORKSPACE_STORAGE_KEY = 'let_timer_active_workspace';

export function getStoredToken(): string | null {
  return localStorage.getItem(TOKEN_STORAGE_KEY);
}

export function setStoredToken(token: string | null): void {
  if (token) {
    localStorage.setItem(TOKEN_STORAGE_KEY, token);
  } else {
    localStorage.removeItem(TOKEN_STORAGE_KEY);
  }
}

export function getStoredWorkspace(): string | null {
  return localStorage.getItem(WORKSPACE_STORAGE_KEY);
}

export function setStoredWorkspace(wsId: string | null): void {
  if (wsId) {
    localStorage.setItem(WORKSPACE_STORAGE_KEY, wsId);
  } else {
    localStorage.removeItem(WORKSPACE_STORAGE_KEY);
  }
}

export const apiClient = createClient<paths>({
  baseUrl: '',
});

// Configure middleware for Authorization and Workspace headers
apiClient.use({
  async onRequest({ request }) {
    const token = getStoredToken();
    if (token) {
      request.headers.set('Authorization', `Bearer ${token}`);
    }
    request.headers.set('X-Let-Timer-Protocol', '1');

    const ws = getStoredWorkspace();
    if (ws) {
      request.headers.set('X-Let-Timer-Workspace', ws);
    }
    return request;
  },
  async onResponse({ response }) {
    if (response.status === 401) {
      // Clear token on 401
      setStoredToken(null);
      window.dispatchEvent(new CustomEvent('let-timer-auth-expired'));
    }
    return response;
  },
});
