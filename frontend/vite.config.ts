import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'node:path'

/** E2E：Playwright 注入随机网关端口，避免与本机已占用 8000 冲突 */
const apiProxyTarget =
  process.env.E2E_GATEWAY_URL ?? 'http://127.0.0.1:8000'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      // 必须用 `/api/` 前缀，否则 `/api-keys` 等控制台路由会被误代理到后端
      '/api/': {
        target: apiProxyTarget,
        changeOrigin: true,
        ws: true,
      },
      '/healthz': {
        target: apiProxyTarget,
        changeOrigin: true,
      },
      '/users': {
        target: apiProxyTarget,
        changeOrigin: true,
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
})
