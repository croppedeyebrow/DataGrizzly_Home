/**
 * 개념: 수정본 검사의 TypeScript 쪽 대조.
 * 실행: `node tools/content-core/check.mjs`. 고정 JSON과 같은 거절 코드를 계산한다.
 * 목적: Rust `check_document`와 같은 입력에서 같은 코드가 나오게 한다.
 * 관련: `tools/content-core/src/validate.rs`, `fixtures/contracts/m4-note.json`, `services/api/src/content.rs`.
 */
import { readFileSync } from 'node:fs'

const known = new Set(['paragraph', 'heading', 'list', 'quote', 'table', 'image', 'code', 'math', 'callout', 'related'])
const fields = {
  paragraph: ['id', 'type', 'text'],
  heading: ['id', 'type', 'level', 'text', 'anchor'],
}

/**
 * 개념: 검사 코드만 모은다.
 * 실행: schemaVersion, 모르는 블록, 문단의 추가 필드, 중복 앵커를 본다.
 * 목적: API가 모르는 값을 버리지 않고 거절하는 규칙과 맞춘다.
 * 관련: `services/api/src/content.rs`의 `unknown_type`, `unknown_field`.
 */
export function checkDocument(input) {
  const value = JSON.parse(input)
  const errors = []
  if (value.schemaVersion !== 1) errors.push({ path: 'schemaVersion', code: 'unknown_version' })
  const anchors = []
  for (const [index, block] of (value.blocks ?? []).entries()) {
    if (!known.has(block.type)) {
      errors.push({ path: `blocks/${index}/type`, code: 'unknown_type' })
      continue
    }
    const allowed = fields[block.type]
    if (allowed) {
      for (const key of Object.keys(block)) {
        if (!allowed.includes(key)) errors.push({ path: `blocks/${index}/${key}`, code: 'unknown_field' })
      }
    }
    if (block.type === 'heading' && anchors.includes(block.anchor)) {
      errors.push({ path: `blocks/${index}/anchor`, code: 'duplicate_anchor' })
    }
    if (block.type === 'heading') anchors.push(block.anchor)
  }
  return errors
}

/**
 * 개념: 저장된 앵커를 유지하고 중복만 번호를 붙인다.
 * 실행: 첫 앵커는 원문 그대로, 두 번째는 `-2`다.
 * 목적: 편집기가 저장한 anchor와 추출 결과가 같게 한다.
 * 관련: `apps/admin/src/editor/Editor.tsx`의 `anchorFrom`, `tools/content-core/src/walk.rs`.
 */
export function sectionAnchors(input) {
  const value = JSON.parse(input)
  const used = []
  const anchors = []
  for (const block of value.blocks ?? []) {
    if (block.type !== 'heading') continue
    let anchor = block.anchor || 'section'
    if (used.includes(anchor)) anchor = `${block.anchor}-2`
    used.push(anchor)
    anchors.push(anchor)
  }
  return anchors.length ? anchors : ['section']
}

const fixture = readFileSync(new URL('../../fixtures/contracts/m4-note.json', import.meta.url), 'utf8')
const passed = checkDocument(fixture)
const version = checkDocument(fixture.replace('"schemaVersion": 1', '"schemaVersion": 2'))
const extra = checkDocument(fixture.replace('"type": "paragraph"', '"type": "paragraph", "extra": true'))
const widget = checkDocument(fixture.replace('"type": "paragraph"', '"type": "widget"'))
const anchors = sectionAnchors(fixture)
if (passed.length !== 0) throw new Error('fixture should pass')
if (!version.some((error) => error.code === 'unknown_version')) throw new Error('version')
if (!extra.some((error) => error.code === 'unknown_field')) throw new Error('field')
if (!widget.some((error) => error.code === 'unknown_type')) throw new Error('type')
if (anchors[0] !== 'field-note') throw new Error('anchor')
console.log('content-core-ok')
