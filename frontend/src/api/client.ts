import axios from 'axios';
import { AUTH_LOGOUT_EVENT, AUTH_TOKEN_STORAGE_KEY } from '../hooks/useAuth';

type HttpMethod = 'get' | 'post' | 'put' | 'delete';

// Configuration resolution order mirrors the README.md#environment-configuration
// guidance: prefer runtime overrides injected on `window.__FSI_EXPENSES_CONFIG__`,
// then fall back to the `<meta name="fsi-expenses-api-base">` tag emitted by the
// hosting HTML, and finally the build-time Vite constant `__API_BASE__`. Update
// README and this comment together when adding new config sources.
const apiBase = (window.__FSI_EXPENSES_CONFIG__?.apiBaseUrl as string | undefined) ??
  document.querySelector('meta[name="fsi-expenses-api-base"]')?.getAttribute('content') ??
  __API_BASE__ ?? '';

const ensureTrailingSlash = (value: string) => (value.endsWith('/') ? value : `${value}/`);

const resolveBaseUrl = () => {
  const trimmed = apiBase.trim();
  if (trimmed.length === 0) {
    return '/api/';
  }
  return ensureTrailingSlash(trimmed);
};

const isAbsoluteUrl = (url: string) => /^[a-z][a-z\d+\-.]*:/.test(url) || url.startsWith('//');

const normalizeRelativeUrl = (url: string) => url.replace(/^\/+/u, '');

const client = axios.create({
  baseURL: resolveBaseUrl(),
  withCredentials: true
});

client.interceptors.request.use((config) => {
  const token = localStorage.getItem(AUTH_TOKEN_STORAGE_KEY);
  if (token) {
    config.headers = config.headers ?? {};
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

client.interceptors.response.use(
  (response) => response,
  (error) => {
    if (axios.isAxiosError(error) && error.response?.status === 401) {
      if (localStorage.getItem(AUTH_TOKEN_STORAGE_KEY)) {
        localStorage.removeItem(AUTH_TOKEN_STORAGE_KEY);
        window.dispatchEvent(new Event(AUTH_LOGOUT_EVENT));
      }
    }
    return Promise.reject(error);
  }
);

export const request = async <T>(method: HttpMethod, url: string, data?: unknown) => {
  const finalUrl = isAbsoluteUrl(url) ? url : normalizeRelativeUrl(url);
  const response = await client.request<T>({ method, url: finalUrl, data });
  return response.data;
};

export default client;
