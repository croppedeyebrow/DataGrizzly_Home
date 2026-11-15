/**
 * 기능: 검색 화면 상태 전이.
 * 실행: `node --experimental-strip-types apps/web/src/scripts/explore.test.ts`
 * 목적: 429, 503, 결과 없음, 늦은 응답이 서로 다른 결과로 남는지 본다.
 * 연관: `apps/web/src/scripts/explore.ts`.
 */
import assert from 'node:assert/strict'
import { acceptResponse, viewFor } from './explore.ts'

assert.equal(viewFor(200, 2), 'results')
assert.equal(viewFor(200, 0), 'empty')
assert.equal(viewFor(429, 0), 'limited')
assert.equal(viewFor(503, 0), 'unavailable')
assert.equal(viewFor(422, 0), 'invalid')
assert.equal(acceptResponse(2, 2), true)
assert.equal(acceptResponse(1, 2), false)
console.log('explore-ok')
