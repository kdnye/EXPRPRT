declare module '*.woff2';
declare module '*.woff';

declare global {
  interface Window {
    __FSI_EXPENSES_CONFIG__?: {
      apiBaseUrl?: string;
    };
  }

  const __API_BASE__: string;
  const __BUILD_VERSION__: string;
}

export {};
