import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// 개념: 관리자 개발 서버.
// 목적: 루프백 5173과 자산 기준 /admin/만 연다. 공개 웹 4321과 같이 띄우지 않는다.
// 사용처: apps/admin 의 dev, build, preview.
export default defineConfig({
  base: '/admin/',
  plugins: [react()],
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
  },
  preview: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
  },
})
