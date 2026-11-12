/**
 * 개념: 공개 웹 정적 빌드.
 * 목적: Tailwind CSS를 만든 뒤 경로별 HTML과 수집 문서를 출력 폴더에 쓴다. DG_WEB_OUT이 있으면 그 경로이고, 없으면 dist다.
 * 사용처: package.json 의 build.
 */
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { build, createServer } from 'vite'

const root = path.dirname(fileURLToPath(import.meta.url))
const outDir = process.env.DG_WEB_OUT ? path.resolve(process.env.DG_WEB_OUT) : path.join(root, 'dist')
await build({
  configFile: path.join(root, 'vite.config.ts'),
  build: { outDir, emptyOutDir: true },
})

const assets = path.join(outDir, 'assets')
const cssName = readdirSync(assets).find((name) => name.endsWith('.css'))
if (!cssName) throw new Error('Tailwind CSS was not emitted')
const css = readFileSync(path.join(assets, cssName), 'utf8')

const server = await createServer({
  configFile: path.join(root, 'vite.config.ts'),
  server: { middlewareMode: true },
  appType: 'custom',
})
try {
  const mod = await server.ssrLoadModule('/src/prerender.tsx')
  const files = mod.renderSite(css)
  for (const file of files) {
    const target = path.join(outDir, file.path)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, file.body)
  }
} finally {
  await server.close()
}
