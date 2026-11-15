/**
 * 기능: 릴리스 확인 명령.
 * 실행: `node infra/ops/verify-release.mjs --health <url> --release-dir <dir> --expect <id> --timeout-ms <n>`
 * 목적: JSON을 출력하고, 실패하면 종료 코드 1로 끝낸다.
 * 연관: `infra/ops/verify.mjs`, `services/api/src/snapshot.rs`의 `release.txt`.
 */
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { verifyRelease } from './verify.mjs'

const args = new Map()
for (let index = 2; index < process.argv.length; index += 2) {
  args.set(process.argv[index], process.argv[index + 1] ?? '')
}

const healthUrl = args.get('--health') ?? ''
const releaseDir = args.get('--release-dir') ?? ''
const expected = args.get('--expect') ?? ''
const timeoutMs = Number(args.get('--timeout-ms') ?? '1500')

const checked = await checkRelease({ healthUrl, releaseDir, expected, timeoutMs })
process.stdout.write(`${JSON.stringify(checked)}\n`)
process.exit(checked.exitCode)

/**
 * 기능: 헬스와 릴리스 파일을 읽는다.
 * 목적: 명령과 테스트가 같은 판단을 쓰게 한다.
 * 연관: `verifyRelease`.
 */
export async function checkRelease({ healthUrl, releaseDir, expected, timeoutMs }) {
  let healthUp = false
  let timedOut = false
  let connectionFailed = false
  let logText = ''
  try {
    const response = await fetch(healthUrl, { signal: AbortSignal.timeout(timeoutMs) })
    const body = await response.text()
    logText = body
    healthUp = response.ok && body.includes('"status":"UP"')
  } catch (error) {
    logText = String(error?.name ?? error)
    timedOut = error?.name === 'TimeoutError' || error?.name === 'AbortError'
    connectionFailed = !timedOut
  }
  let marker = ''
  let manifestReleaseId = ''
  try {
    marker = readFileSync(path.join(releaseDir, 'release.txt'), 'utf8')
    manifestReleaseId = JSON.parse(readFileSync(path.join(releaseDir, 'manifest.json'), 'utf8')).releaseId ?? ''
  } catch (error) {
    logText = `${logText}\n${error?.code ?? 'release_read_failed'}`
  }
  return verifyRelease({ healthUp, marker, expectedReleaseId: expected, manifestReleaseId, timedOut, connectionFailed, logText })
}
