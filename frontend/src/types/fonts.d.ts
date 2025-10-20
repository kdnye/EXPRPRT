declare module '*.woff2';
declare module '*.woff';

declare global {
  interface Window {
    __FSI_EXPENSES_CONFIG__?: {
      apiBaseUrl?: string;
      authBypass?: {
        enabled?: boolean;
        role?: 'employee' | 'manager' | 'finance' | 'admin';
      };
    };
  }

  const __API_BASE__: string;
  const __BUILD_VERSION__: string;

  interface ImportMetaEnv {
    readonly VITE_AUTH_BYPASS?: string;
    readonly VITE_AUTH_BYPASS_ROLE?: string;
  }

  interface ImportMeta {
    readonly env: ImportMetaEnv;
  }
}

export {};
