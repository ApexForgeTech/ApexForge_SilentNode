import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const apiTarget = env.SILENTNODE_API_TARGET || 'http://localhost:3030'

  return {
    plugins: [react()],
    build: {
      chunkSizeWarningLimit: 1600,
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (/node_modules\/(react|react-dom)\//.test(id)) {
              return 'react-vendor'
            }
            if (/node_modules\/axios\//.test(id)) {
              return 'http-vendor'
            }
          },
        },
      },
    },
    server: {
      port: 5173,
      proxy: {
        '/api': {
          target: apiTarget,
          changeOrigin: true,
          rewrite: (path) => path.replace(/^\/api/, ''),
        },
      },
    },
  }
})
