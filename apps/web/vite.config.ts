import { existsSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig, type Connect, type Plugin, type PreviewServer, type ViteDevServer } from 'vite'

const webRoot = path.dirname(fileURLToPath(import.meta.url))

function rewritePreviewPath(req: Connect.IncomingMessage) {
  const raw = req.url ?? '/'
  const query = raw.includes('?') ? raw.slice(raw.indexOf('?')) : ''
  const pathOnly = raw.split('?')[0] || '/'
  if (pathOnly === '/' || pathOnly.includes('.')) return
  const relative = pathOnly.replace(/^\//, '').replace(/\/$/, '')
  const index = path.join(webRoot, 'dist', relative, 'index.html')
  req.url = existsSync(index) ? `${pathOnly.endsWith('/') ? pathOnly : `${pathOnly}/`}${query}` : `/404.html${query}`
}

/**
 * 개념: 공개 웹 개발 서버.
 * 목적: 루프백 4321에서 React 페이지를 HTML로 돌려준다.
 * 사용처: apps/web 의 dev, preview, build.
 */
function directoryIndex(): Plugin {
  return {
    name: 'directory-index',
    configurePreviewServer(server: PreviewServer) {
      server.middlewares.use((req, _res, next) => {
        rewritePreviewPath(req)
        next()
      })
    },
  }
}

function siteDev(): Plugin {
  return {
    name: 'site-dev',
    configureServer(server: ViteDevServer) {
      server.middlewares.use(async (req, res, next) => {
        const url = req.url ?? '/'
        const path = url.split('?')[0] ?? '/'
        if (
          path.startsWith('/@') ||
          path.startsWith('/src') ||
          path.startsWith('/node_modules') ||
          path.startsWith('/media') ||
          path.startsWith('/favicon') ||
          path.startsWith('/robots')
        ) {
          next()
          return
        }
        const mod = (await server.ssrLoadModule('/src/prerender.tsx')) as {
          renderRequest: (input: string) => { body: string; kind: 'html' | 'xml' } | null
        }
        const page = mod.renderRequest(url)
        if (!page) {
          next()
          return
        }
        res.statusCode = path === '/404' ? 404 : 200
        res.setHeader('content-type', page.kind === 'xml' ? 'application/xml; charset=utf-8' : 'text/html; charset=utf-8')
        res.end(page.body)
      })
    },
  }
}

export default defineConfig({
  plugins: [react(), tailwindcss(), directoryIndex(), siteDev()],
  server: {
    host: '127.0.0.1',
    port: 4321,
    strictPort: true,
    fs: { allow: ['../..'] },
  },
  preview: {
    host: '127.0.0.1',
    port: 4321,
    strictPort: true,
  },
  build: {
    manifest: true,
    rollupOptions: {
      input: 'src/boot.ts',
      output: {
        entryFileNames: 'menu.js',
        assetFileNames: 'assets/[name]-[hash][extname]',
      },
    },
  },
})
