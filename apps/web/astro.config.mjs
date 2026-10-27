// @ts-check
import { defineConfig } from 'astro/config';

// 개념: 공개 웹 개발 서버와 저장소 밖 fixture 읽기.
// 목적: 루프백 4321만 열고, 홈이 저장소 루트의 예시 파일을 가져오게 한다.
// 사용처: apps/web 의 dev, build, preview.
export default defineConfig({
  server: {
    host: '127.0.0.1',
    port: 4321,
  },
  vite: {
    server: {
      fs: {
        allow: ['../..'],
      },
    },
  },
});
