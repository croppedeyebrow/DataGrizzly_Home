/**
 * 개념: 후보 릴리스용 React 정적 빌드.
 * 목적: 활성 dist를 그대로 고치지 않고, 지정한 릴리스 폴더에만 빌드 결과를 둔다.
 * 실행: DG_CONTENT_BIN이 있으면 후보 빌드 전에 `dg-content validate`를 실행하고, 0이 아니면 폴더를 만들기 전에 멈춘다.
 * 사용처: DG_PUBLISH_REACT=1 일 때 발행 확인. 인자: 스냅샷 JSON 경로, 출력 폴더, releaseId. 관련: content-gate.mjs, tools/content-cli.
 */
import { createHash } from 'node:crypto'
import { buildMayContinue } from './content-gate.mjs'
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
const snapshotPath = path.resolve(process.argv[2] ?? '')
const outDir = path.resolve(process.argv[3] ?? '')
const releaseId = process.argv[4] ?? ''
const activeDist = path.resolve(repo, 'apps/web/dist')

if (!snapshotPath || !outDir || !releaseId) {
  console.error('usage: node infra/publish/build-candidate.mjs <snapshot.json> <outDir> <releaseId>')
  process.exit(2)
}
if (outDir === activeDist) {
  console.error('candidate output must not be the active dist')
  process.exit(2)
}

const contentBin = process.env.DG_CONTENT_BIN ?? ''
let contentCheck = { called: false, reason: 'binary_unset' }
if (contentBin) {
  const checked = spawnSync(contentBin, ['validate', snapshotPath], { encoding: 'utf8' })
  if (!buildMayContinue(checked.status)) {
    process.stderr.write(checked.stderr || 'content check failed\n')
    process.exit(checked.status ?? 1)
  }
  const binHash = createHash('sha256').update(readFileSync(contentBin)).digest('hex')
  contentCheck = { called: true, status: 0, binHash }
}

mkdirSync(outDir, { recursive: true })
const lockfile = path.join(repo, 'yarn.lock')
const lockHash = createHash('sha256').update(readFileSync(lockfile)).digest('hex')
const built = spawnSync(process.execPath, [path.join(repo, 'apps/web/build.mjs')], {
  cwd: repo,
  env: { ...process.env, DG_WEB_OUT: outDir },
  stdio: 'inherit',
})
if (built.status !== 0) process.exit(built.status ?? 1)

const marker = `dg-release:${releaseId}`
const indexPath = path.join(outDir, 'index.html')
const index = readFileSync(indexPath, 'utf8')
if (!index.includes(marker)) {
  writeFileSync(indexPath, `${index}\n<!-- ${marker} -->\n`)
}
writeFileSync(path.join(outDir, 'release.txt'), `${releaseId}\n`)
writeFileSync(
  path.join(outDir, 'manifest.json'),
  JSON.stringify({ schemaVersion: 1, releaseId, lockfileHash: lockHash, builder: 'react-static', contentCheck }),
)
console.log(JSON.stringify({ releaseId, lockfileHash: lockHash, builder: 'react-static' }))
