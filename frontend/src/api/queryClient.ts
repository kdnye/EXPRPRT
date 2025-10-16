import { QueryClient } from '@tanstack/react-query';
import type { AxiosError } from 'axios';

const isClientError = (error: unknown) => {
  const axiosError = error as AxiosError | undefined;
  const status = axiosError?.response?.status;
  return typeof status === 'number' && status >= 400 && status < 500;
};

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 60_000,
      gcTime: 5 * 60_000,
      retry: (failureCount, error) => {
        if (isClientError(error)) {
          return false;
        }
        return failureCount < 2;
      },
      refetchOnWindowFocus: false
    },
    mutations: {
      retry: 0
    }
  }
});

export default queryClient;
