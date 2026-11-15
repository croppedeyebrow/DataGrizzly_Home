/**
 * 개념: 브라우저 모듈 실패가 저장을 막지 않는지 확인한다.
 * 실행: `node --experimental-strip-types apps/admin/src/editor/wasm-check.test.ts`
 * 목적: 시간 초과와 불러오기 실패가 null이고, 성공만 값을 돌려주는지 본다.
 * 관련: `apps/admin/src/editor/wasm-check.ts`, `apps/admin/src/editor/Editor.tsx`.
 */
import assert from 'node:assert/strict'
import { loadOptionalCheck } from './wasm-check.ts'

const missing = await loadOptionalCheck(async () => {
  throw new Error('missing')
}, 50)
assert.equal(missing, null)

const slow = await loadOptionalCheck(async () => {
  await new Promise((resolve) => setTimeout(resolve, 200))
  return 'wasm'
}, 20)
assert.equal(slow, null)

const ready = await loadOptionalCheck(async () => 'wasm', 50)
assert.equal(ready, 'wasm')
console.log('wasm-check-ok')
