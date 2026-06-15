import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// Proxy the gateway's REST + WebSocket routes so the app can use relative URLs in dev,
// matching the nginx setup used in production (docker-compose).
const GATEWAY = 'http://127.0.0.1:8080'
const proxied = ['/orders', '/book', '/trades', '/sim', '/health', '/metrics']

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      ...Object.fromEntries(
        proxied.map((p) => [p, { target: GATEWAY, changeOrigin: true }]),
      ),
      '/ws': { target: 'ws://127.0.0.1:8080', ws: true },
    },
  },
})
