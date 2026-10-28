/**
 * 개념: 글 블록을 화면에 안전한 값으로 나눈다.
 * 목적: 모르는 블록은 오류로 남기고, 코드와 수식과 주소는 실행하지 않는다.
 * 사용처: scripts/public-research.mjs, scripts/check-m1-research.mjs, apps/web/src/components/BlockList.astro가 같은 종류 이름을 쓴다.
 * schema 1의 코드 블록에는 파일명 필드가 없다. headingPath는 제목 anchor를 위쪽부터 쌓은 값이며 나중 검색 절이 같은 anchor를 쓰게 둔다.
 */
const BLOCK_TYPES = new Set([
  'paragraph',
  'heading',
  'list',
  'quote',
  'table',
  'image',
  'code',
  'math',
  'callout',
  'related',
])

const MATH_COMMANDS = new Set([
  'frac',
  'sqrt',
  'sum',
  'int',
  'alpha',
  'beta',
  'gamma',
  'times',
  'cdot',
  'left',
  'right',
  'text',
  'mathrm',
  'le',
  'ge',
  'ne',
  'pm',
  'infty',
])

const REJECTED_MATH = '이 수식은 허용되지 않아 표시하지 않는다.'
const UNKNOWN_BLOCK = '지원하지 않는 블록이다.'

export function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
}

function httpsLinks(text) {
  const parts = []
  const pattern = /https:\/\/[^\s<>"']+/g
  let last = 0
  for (const match of text.matchAll(pattern)) {
    const href = match[0]
    if (match.index > last) parts.push({ kind: 'text', text: text.slice(last, match.index) })
    if (/^https:\/\/[^\s]+$/.test(href) && !/javascript\s*:/i.test(href)) {
      parts.push({ kind: 'link', href })
    } else {
      parts.push({ kind: 'text', text: href })
    }
    last = match.index + href.length
  }
  if (last < text.length) parts.push({ kind: 'text', text: text.slice(last) })
  if (parts.length === 0) parts.push({ kind: 'text', text })
  return parts
}

function mathAllowed(tex) {
  if (/[<>]/.test(tex) || /javascript\s*:/i.test(tex)) return false
  const commands = [...tex.matchAll(/\\([A-Za-z]+)/g)].map((match) => match[1])
  return commands.every((command) => MATH_COMMANDS.has(command))
}

function imageName(url) {
  try {
    return new URL(url).pathname.split('/').pop() ?? ''
  } catch {
    return ''
  }
}

export function renderDocument(blocks, mediaFiles = new Set()) {
  const errors = []
  const rendered = []
  const anchors = new Set()
  const stack = []
  const source = Array.isArray(blocks) ? blocks : []

  for (const block of source) {
    const type = block?.type
    if (!BLOCK_TYPES.has(type)) {
      errors.push(`unknown-block:${type ?? 'missing'}`)
      rendered.push({ kind: 'error', message: UNKNOWN_BLOCK })
      continue
    }

    if (type === 'heading') {
      const anchor = typeof block.anchor === 'string' ? block.anchor : ''
      if (!anchor || anchors.has(anchor)) {
        errors.push(`duplicate-anchor:${anchor || 'missing'}`)
        rendered.push({ kind: 'error', message: '제목 anchor가 없거나 중복이다.' })
        continue
      }
      anchors.add(anchor)
      const level = block.level === 3 ? 3 : 2
      while (stack.length > 0 && stack[stack.length - 1].level >= level) stack.pop()
      stack.push({ level, anchor })
      rendered.push({
        kind: 'heading',
        level,
        text: block.text,
        anchor,
        headingPath: stack.map((item) => item.anchor),
      })
      continue
    }

    if (type === 'paragraph') {
      rendered.push({ kind: 'paragraph', parts: httpsLinks(block.text ?? '') })
      continue
    }

    if (type === 'list') {
      rendered.push({
        kind: 'list',
        ordered: block.ordered === true,
        items: Array.isArray(block.items) ? block.items : [],
      })
      continue
    }

    if (type === 'quote') {
      rendered.push({ kind: 'quote', text: block.text ?? '', citation: block.citation ?? '' })
      continue
    }

    if (type === 'table') {
      rendered.push({
        kind: 'table',
        caption: block.caption ?? '',
        header: Array.isArray(block.header) ? block.header : [],
        rows: Array.isArray(block.rows) ? block.rows : [],
      })
      continue
    }

    if (type === 'image') {
      const name = imageName(block.url ?? '')
      rendered.push({
        kind: 'image',
        alt: block.alt ?? '',
        imageName: name,
        missing: !name || !mediaFiles.has(name),
      })
      continue
    }

    if (type === 'code') {
      rendered.push({
        kind: 'code',
        language: block.language ?? '',
        filename: null,
        text: block.text ?? '',
      })
      continue
    }

    if (type === 'math') {
      const tex = block.tex ?? ''
      if (!mathAllowed(tex)) {
        rendered.push({ kind: 'math', state: 'rejected', message: REJECTED_MATH })
      } else {
        rendered.push({ kind: 'math', state: 'shown', tex })
      }
      continue
    }

    if (type === 'callout') {
      rendered.push({ kind: 'callout', tone: block.tone === 'note' ? 'note' : 'warning', text: block.text ?? '' })
      continue
    }

    rendered.push({ kind: 'related', entryId: block.entryId ?? '' })
  }

  return { ok: errors.length === 0, errors, blocks: rendered }
}

export function toPreviewHtml(document) {
  return document.blocks.map((block) => {
    if (block.kind === 'code') {
      return `<p>언어 ${escapeHtml(block.language)}. 파일명이 아직 없다.</p><pre class="code-block"><code>${escapeHtml(block.text)}</code></pre>`
    }
    if (block.kind === 'image') {
      return block.missing
        ? `<p>${escapeHtml(block.alt)} 파일을 찾지 못했다.</p>`
        : `<img alt="${escapeHtml(block.alt)}" />`
    }
    if (block.kind === 'math' && block.state === 'shown') {
      return `<pre class="math-block"><code>${escapeHtml(block.tex)}</code></pre>`
    }
    if (block.kind === 'math') return `<p>${escapeHtml(block.message)}</p>`
    if (block.kind === 'error') return `<p>${escapeHtml(block.message)}</p>`
    if (block.kind === 'table') return '<div class="table-scroll"><table></table></div>'
    if (block.kind === 'paragraph') {
      return block.parts.map((part) => part.kind === 'link'
        ? `<a href="${escapeHtml(part.href)}">${escapeHtml(part.href)}</a>`
        : escapeHtml(part.text)).join('')
    }
    return ''
  }).join('')
}
