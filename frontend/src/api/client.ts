import axios from 'axios';

type HttpMethod = 'get' | 'post' | 'put' | 'delete';

// Configuration resolution order mirrors the README.md#environment-configuration
// guidance: prefer runtime overrides injected on `window.__FSI_EXPENSES_CONFIG__`,
// then fall back to the `<meta name="fsi-expenses-api-base">` tag emitted by the
// hosting HTML, and finally the build-time Vite constant `__API_BASE__`. Update
// README and this comment together when adding new config sources.
const apiBase = (window.__FSI_EXPENSES_CONFIG__?.apiBaseUrl as string | undefined) ??
  document.querySelector('meta[name="fsi-expenses-api-base"]')?.getAttribute('content') ??
  __API_BASE__ ?? '';

const client = axios.create({
  baseURL: apiBase || '/api',
  withCredentials: true
});

export const request = async <T>(method: HttpMethod, url: string, data?: unknown) => {
  const response = await client.request<T>({ method, url, data });
  return response.data;
};

export default client;
