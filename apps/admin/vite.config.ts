import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// 개념: 관리자 개발 서버.
// 목적: 루프백 5173과 자산 기준 /admin/만 연다. 공개 웹 4321과 같이 띄우지 않는다.
//       Tailwind 플러그인으로 공개 웹과 같은 자수정 테마(design-tokens/theme.css)를 쓴다.
// 사용처: apps/admin 의 dev, build, preview.
export default defineConfig({
  base: '/admin/',
  plugins: [react(), tailwindcss()],
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': 'http://127.0.0.1:8080',
    },
  },
  preview: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': 'http://127.0.0.1:8080',
    },
  },
})
