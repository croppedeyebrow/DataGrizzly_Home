/**
 * 기능: 배포 판단 테스트.
 * 실행: `node --test infra/ops/ops.test.mjs`
 * 목적: 실패 종료 코드, 비밀값 제거, digest, 격리 복원, 출시 보류를 확인한다.
 * 연관: `verify.mjs`, `promote.mjs`, `deploy.mjs`, `backup.mjs`, `release-gate.mjs`.
 */
import assert from 'node:assert/strict'
import { fileURLToPath } from 'node:url'
import { createServer } from 'node:http'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import test from 'node:test'
import { spawn } from 'node:child_process'
import { backupTargets, missingMedia, recoveryComparison, restoreAllowed, rollbackDecision } from './backup.mjs'
import { databaseLoopbackOnly, deployPlan } from './deploy.mjs'
import { promote } from './promote.mjs'
import { budgetSignal, releaseGate } from './release-gate.mjs'
import { redact, verifyRelease } from './verify.mjs'

const digest = `sha256:${'a'.repeat(64)}`

test('failed release checks exit nonzero and redact secrets', () => {
  const wrong = verifyRelease({
    healthUp: true,
    marker: 'other',
    expectedReleaseId: 'rel-1',
    manifestReleaseId: 'rel-1',
    timedOut: false,
    connectionFailed: false,
    logText: 'password=secret-value',
  })
  assert.equal(wrong.exitCode, 1)
  assert.equal(wrong.code, 'release_mismatch')
  assert.equal(wrong.log.includes('secret-value'), false)
  const timed = verifyRelease({ healthUp: false, marker: '', expectedReleaseId: 'rel-1', manifestReleaseId: '', timedOut: true, connectionFailed: false, logText: '' })
  assert.equal(timed.code, 'timeout')
  assert.equal(redact('POSTGRES_PASSWORD=hunter2').includes('hunter2'), false)
})

test('only a matching digest can be promoted', () => {
  assert.equal(promote({ digest: 'latest', builtDigest: digest }).code, 'digest_required')
  assert.equal(promote({ digest, builtDigest: `sha256:${'b'.repeat(64)}` }).code, 'digest_mismatch')
  assert.equal(promote({ digest, builtDigest: digest }).exitCode, 0)
})

test('schema rollback stays manual and the database port stays on loopback', async () => {
  assert.equal(deployPlan({ migrationCompatible: false, smokeOk: true }).rollbackSchema, false)
  assert.equal(deployPlan({ migrationCompatible: true, smokeOk: false }).rollbackImage, true)
  assert.equal(deployPlan({ migrationCompatible: true, smokeOk: false }).rollbackSchema, false)
  const compose = await readFile(new URL('../local/compose.yaml', import.meta.url), 'utf8')
  assert.equal(databaseLoopbackOnly(compose).public, false)
})

test('restore refuses the production database and keeps the denylist in scope', () => {
  assert.deepEqual(backupTargets(), ['database', 'media', 'entry_denylist', 'release_manifest', 'workflow_checkpoint'])
  assert.equal(restoreAllowed({ sourceDb: 'datagrizzly', targetDb: 'datagrizzly', sourceRoot: 'D:/backup', targetRoot: 'E:/restore' }).code, 'same_database')
  assert.equal(restoreAllowed({ sourceDb: 'datagrizzly', targetDb: 'datagrizzly_restore', sourceRoot: 'D:/data', targetRoot: 'D:/data' }).code, 'same_disk')
  assert.equal(restoreAllowed({ sourceDb: 'datagrizzly', targetDb: 'datagrizzly_restore', sourceRoot: 'D:/data', targetRoot: 'E:/restore' }).ok, true)
  assert.deepEqual(missingMedia(['a', 'b'], ['a']), ['b'])
  assert.equal(rollbackDecision('schema').automatic, false)
  assert.equal(recoveryComparison({ restoreMinutes: null, dataAgeHours: null }).met, false)
})

test('release gate does not call a missing model a defect or a finished launch', () => {
  const gate = releaseGate({ searchLive: false, backupMeasured: false, publicUrl: '' })
  assert.equal(gate.modelOffIsDefect, false)
  assert.equal(gate.launchComplete, false)
  assert.equal(gate.highAvailability, false)
  assert.equal(budgetSignal('alert').stopsSpend, false)
  assert.equal(budgetSignal('hard_cap').stopsSpend, true)
})

test('the api image definition does not copy an env file', async () => {
  const docker = await readFile(new URL('../../services/api/Dockerfile', import.meta.url), 'utf8')
  assert.match(docker, /USER app/)
  assert.match(docker, /actuator\/health/)
  assert.equal(docker.includes('.env'), false)
})

test('the verify command fails closed when the health port is closed', async () => {
  const dir = await mkdtemp(path.join(tmpdir(), 'dg-ops-'))
  await writeFile(path.join(dir, 'release.txt'), 'rel-1\n')
  await writeFile(path.join(dir, 'manifest.json'), JSON.stringify({ releaseId: 'rel-1' }))
  const output = await runVerify(['--health', 'http://127.0.0.1:9/actuator/health', '--release-dir', dir, '--expect', 'rel-1', '--timeout-ms', '300'])
  assert.equal(output.status, 1)
  assert.equal(JSON.parse(output.stdout).ok, false)
  await rm(dir, { recursive: true })
})

test('the verify command accepts a matching local health response', async () => {
  const server = createServer((_request, response) => {
    response.writeHead(200, { 'content-type': 'application/json' })
    response.end('{"groups":["liveness","readiness"],"status":"UP"}')
  })
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
  const { port } = server.address()
  const dir = await mkdtemp(path.join(tmpdir(), 'dg-ops-'))
  await writeFile(path.join(dir, 'release.txt'), 'rel-1\n')
  await writeFile(path.join(dir, 'manifest.json'), JSON.stringify({ releaseId: 'rel-1' }))
  const output = await runVerify(['--health', `http://127.0.0.1:${port}/actuator/health`, '--release-dir', dir, '--expect', 'rel-1', '--timeout-ms', '1000'])
  server.close()
  assert.equal(output.status, 0)
  assert.equal(JSON.parse(output.stdout).code, 'ok')
  await rm(dir, { recursive: true })
})

function runVerify(args) {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, [fileURLToPath(new URL('./verify-release.mjs', import.meta.url)), ...args], { windowsHide: true })
    let stdout = ''
    child.stdout.setEncoding('utf8')
    child.stdout.on('data', (chunk) => { stdout += chunk })
    child.on('exit', (status) => resolve({ status, stdout }))
  })
}
