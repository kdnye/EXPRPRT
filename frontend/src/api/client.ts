import axios from 'axios';

type HttpMethod = 'get' | 'post' | 'put' | 'delete';

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
