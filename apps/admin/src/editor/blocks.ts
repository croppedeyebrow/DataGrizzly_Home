/**
 * 개념: 편집 화면의 블록 값.
 * 목적: 화면 상태가 서버에 저장되는 블록 객체와 같게 한다.
 * 사용처: 편집 화면. 서버가 다시 검사한다.
 */

export type Block = Record<string, unknown>

export type EntryDocument = {
  schemaVersion?: number
  blocks?: Block[]
  metadata?: Record<string, unknown>
  title?: string
}

/**
 * 개념: 빈 블록 하나를 만든다.
 * 목적: 편집기가 별도 문서 형식을 만들지 않고 계약 필드로 시작한다.
 * 사용처: 블록 추가 버튼.
 */
export function blankBlock(type: string): Block {
  const id = crypto.randomUUID()
  if (type === 'heading') return { id, type, level: 2, text: '', anchor: 'section' }
  if (type === 'list') return { id, type, ordered: false, items: [''] }
  if (type === 'quote') return { id, type, text: '' }
  if (type === 'table') return { id, type, header: ['열'], rows: [['']] }
  if (type === 'image') return { id, type, alt: '', url: 'https://' }
  if (type === 'code') return { id, type, language: 'text', text: '' }
  if (type === 'math') return { id, type, tex: '' }
  if (type === 'callout') return { id, type, tone: 'note', text: '' }
  if (type === 'related') return { id, type, entryId: '' }
  return { id, type: 'paragraph', text: '' }
}

/**
 * 개념: 블록을 비교용 문장으로 바꾼다.
 * 목적: 충돌 화면에서 서버 본문을 읽기 전용으로 보여 준다.
 * 사용처: 409 응답의 document.blocks.
 */
export function blockLines(blocks: Block[]): string[] {
  return blocks.map((block) => {
    const type = String(block.type ?? 'unknown')
    const text = ['text', 'tex', 'alt', 'entryId']
      .map((key) => block[key])
      .filter((value) => typeof value === 'string' && value.length > 0)
      .join(' ')
    return text ? `${type}: ${text}` : type
  })
}
