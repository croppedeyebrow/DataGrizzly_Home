/**
 * 개념: 자동저장 상태 기계 시험.
 * 목적: 느린 응답과 409가 로컬 세대를 완료로 덮지 않는지 확인한다.
 * 사용처: `node --experimental-strip-types apps/admin/src/editor/autosave.test.ts`
 */
import assert from 'node:assert/strict'
import { canAutosave, reduceSave, type SaveState } from './autosave.ts'

const loaded = reduceSave(
  { status: 'idle', generation: 0, sentGeneration: 0, expectedVersion: 0, conflictVersion: null },
  { type: 'load', version: 1 },
)

const edited = reduceSave(loaded, { type: 'edit' })
const sending = reduceSave(edited, { type: 'send' })
const typedAgain = reduceSave(sending, { type: 'edit' })
const late = reduceSave(typedAgain, { type: 'saved', generation: sending.sentGeneration, version: 2 })
assert.equal(late.status, 'idle')
assert.equal(late.expectedVersion, 2)
assert.equal(late.generation, typedAgain.generation)
assert.equal(canAutosave(late), true)

const caughtUp = reduceSave(sending, { type: 'saved', generation: sending.sentGeneration, version: 2 })
assert.equal(caughtUp.status, 'saved')
assert.equal(canAutosave(caughtUp), false)

const conflict = reduceSave(sending, { type: 'conflict', version: 4 })
assert.equal(conflict.status, 'conflict')
assert.equal(canAutosave(conflict), false)
const accepted = reduceSave(conflict, { type: 'acceptServerVersion' })
assert.equal(accepted.expectedVersion, 4)
assert.equal(accepted.generation, conflict.generation)
assert.equal(canAutosave(accepted), false)

const failed: SaveState = reduceSave(sending, { type: 'failed' })
assert.equal(failed.status, 'failed')
assert.equal(failed.generation, sending.generation)
