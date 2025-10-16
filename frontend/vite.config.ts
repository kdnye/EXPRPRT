import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '');
  return {
    plugins: [react()],
    define: {
      __API_BASE__: JSON.stringify(env.VITE_API_BASE ?? ''),
      __BUILD_VERSION__: JSON.stringify(process.env.GIT_SHA ?? 'dev')
    }
  };
});
