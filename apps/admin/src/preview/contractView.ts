/**
 * 개념: 계약 블록을 공개 화면의 블록 값으로 바꾼다.
 * 목적: 미리보기가 공개 BlockList를 쓰고, 사용자 HTML은 텍스트로만 남긴다.
 * 사용처: 초안 미리보기와 수정본 비교.
 */
import type { ViewBlock } from '../../../web/src/scripts/research-types'

/**
 * 개념: 계약 JSON의 문자열 필드.
 * 목적: 문자열이 아니면 빈 문자열로 보고, HTML로 해석하지 않는다.
 * 사용처: `contractToView`.
 */
function text(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

/**
 * 개념: 계약 JSON의 문자열 배열.
 * 목적: 목록과 표에서 문자열이 아닌 칸은 빼 공개 렌더러에 넘기지 않는다.
 * 사용처: `contractToView`.
 */
function textList(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : []
}

/**
 * 개념: 저장 블록 배열을 공개 렌더 입력으로 만든다.
 * 목적: 코드와 수식은 실행하지 않고, 모르는 종류는 빈 칸으로 지우지 않는다.
 * 사용처: Preview.
 */
export function contractToView(blocks: unknown): ViewBlock[] {
  // 본문이 배열이 아니면 빈 화면으로 지우지 않고, 지원하지 않는 블록으로 보여 준다.
  if (!Array.isArray(blocks)) return [{ kind: 'unknown', message: '지원하지 않는 블록이다.' }]
  return blocks.map((block) => {
    if (!block || typeof block !== 'object') return { kind: 'unknown', message: '지원하지 않는 블록이다.' }
    const record = block as Record<string, unknown>
    const type = text(record.type)
    // 문단은 링크를 만들지 않고 텍스트 조각 하나로만 넘긴다.
    if (type === 'paragraph') return { kind: 'paragraph', parts: [{ kind: 'text', text: text(record.text) }] }
    // 제목 단계는 2 또는 3만 쓴다. 앵커가 있어야 목차 링크가 된다.
    if (type === 'heading') {
      return { kind: 'heading', level: record.level === 3 ? 3 : 2, text: text(record.text), anchor: text(record.anchor) || undefined }
    }
    if (type === 'list') return { kind: 'list', ordered: record.ordered === true, items: textList(record.items) }
    if (type === 'quote') return { kind: 'quote', text: text(record.text), citation: text(record.citation) || undefined }
    if (type === 'table') {
      const rows = Array.isArray(record.rows) ? record.rows.map((row) => textList(row)) : []
      return { kind: 'table', header: textList(record.header), rows, caption: text(record.caption) || undefined }
    }
    // 공개 렌더러가 그리는 이미지는 예시 표지뿐이다. 그 외 주소는 없는 파일로 표시한다.
    if (type === 'image') {
      return text(record.url).endsWith('example-mark.svg')
        ? { kind: 'image', alt: text(record.alt), imageName: 'example-mark.svg' }
        : { kind: 'image', alt: text(record.alt), missing: true }
    }
    // 코드와 수식 문자열은 실행하지 않는다. 수식은 공개 화면의 '보여 줌' 상태로 텍스트만 그린다.
    if (type === 'code') return { kind: 'code', language: text(record.language) || 'text', text: text(record.text) }
    if (type === 'math') return { kind: 'math', state: 'shown', tex: text(record.tex) }
    if (type === 'callout') return { kind: 'callout', tone: text(record.tone) || 'note', text: text(record.text) }
    // 관련 글의 공개 주소는 여기서 만들지 않는다. 공개 렌더러가 없는 링크 문장을 보여 준다.
    if (type === 'related') return { kind: 'related', href: null, title: text(record.entryId) }
    return { kind: 'unknown', message: '지원하지 않는 블록이다.' }
  })
}
