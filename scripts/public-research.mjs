/**
 * 개념: 공개 노트와 논문, 뉴스를 페이지 값으로 고른다.
 * 목적: 공개 목록 밖의 경계 fixture는 주소를 만들지 않는다. 렌더가 실패한 글도 발행하지 않는다.
 * 사용처: apps/web/src/pages/notes, apps/web/src/pages/research, scripts/check-m1-research.mjs
 */
import { readdirSync, readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { renderDocument } from '../packages/block-renderer/render.mjs'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const roles = new Set(['note', 'paper', 'news'])

function readJson(filePath) {
  return JSON.parse(readFileSync(filePath, 'utf8'))
}

function publicSlug(value) {
  return typeof value === 'string' && /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(value) ? value : ''
}

function paragraphAfter(blocks, anchor) {
  const index = blocks.findIndex((block) => block.anchor === anchor)
  if (index < 0) return ''
  const next = blocks[index + 1]
  return next?.type === 'paragraph' ? next.text : ''
}

export function emptyResearch(loadError) {
  return { loadError, notes: [], research: [] }
}

function accepted(item, entries, revisions) {
  const entry = entries[item.entry]
  const revision = revisions[item.revision]
  if (!entry || !revision) return null
  if (!roles.has(item.role)) return null
  if (entry.example !== true || revision.example !== true) return null
  if (entry.publishedRevisionId !== revision.revisionId) return null
  if (entry.type !== item.role || revision.metadata?.type !== item.role) return null
  const slug = publicSlug(entry.slug)
  if (!slug) return null
  return { role: item.role, entry, revision, slug }
}

function articleHref(role, slug) {
  return role === 'note' ? `/notes/${slug}` : `/research/${slug}`
}

export function selectPublicResearch(manifest, entries, revisions, mediaFiles = new Set()) {
  if (!manifest || manifest.dataset !== 'public-example' || !Array.isArray(manifest.public)) {
    return emptyResearch(false)
  }

  const items = manifest.public
    .map((item) => accepted(item, entries, revisions))
    .filter(Boolean)
  const byEntryId = new Map(items.map((item) => [item.entry.entryId, item]))
  const rejected = []

  const articles = items.map((item) => {
    const rendered = renderDocument(item.revision.blocks ?? [], mediaFiles)
    if (!rendered.ok) {
      rejected.push({ slug: item.slug, errors: rendered.errors })
      return null
    }
    const blocks = item.revision.blocks ?? []
    const viewBlocks = rendered.blocks.map((block) => {
      if (block.kind !== 'related') return block
      const target = byEntryId.get(block.entryId)
      return {
        kind: 'related',
        href: target ? articleHref(target.role, target.slug) : null,
        title: target?.revision.title ?? '',
      }
    })
    return {
      slug: item.slug,
      title: item.revision.title,
      kind: item.role,
      createdAt: item.revision.createdAt,
      updatedAt: item.entry.updatedAt,
      projectId: item.revision.metadata?.projectId ?? null,
      summary: (blocks.find((block) => block.type === 'paragraph') ?? {}).text ?? '',
      source: paragraphAfter(blocks, 'source'),
      claim: paragraphAfter(blocks, 'claim'),
      interpretation: paragraphAfter(blocks, 'interpretation'),
      reproduction: paragraphAfter(blocks, 'reproduction'),
      eventDate: paragraphAfter(blocks, 'event-date'),
      publicationDate: paragraphAfter(blocks, 'publication-date'),
      blocks: viewBlocks,
    }
  }).filter(Boolean)

  const sort = (list) => list.sort((a, b) => {
    if (a.updatedAt !== b.updatedAt) return a.updatedAt < b.updatedAt ? 1 : -1
    if (a.slug < b.slug) return -1
    if (a.slug > b.slug) return 1
    return 0
  })

  return {
    loadError: false,
    rejected,
    notes: sort(articles.filter((item) => item.kind === 'note')),
    research: sort(articles.filter((item) => item.kind === 'paper' || item.kind === 'news')),
  }
}

export function loadPublicResearch(contentRoot = path.join(root, 'fixtures', 'content')) {
  /* 상대 경로는 실행 중인 폴더 기준이다. 페이지는 apps/web에서 ../../fixtures/content를 넘긴다. */
  const resolved = path.isAbsolute(contentRoot) ? contentRoot : path.resolve(process.cwd(), contentRoot)
  try {
    const manifest = readJson(path.join(resolved, 'manifest.json'))
    const entries = {}
    const revisions = {}
    for (const item of manifest.public ?? []) {
      entries[item.entry] = readJson(path.join(resolved, item.entry))
      revisions[item.revision] = readJson(path.join(resolved, item.revision))
    }
    let mediaFiles = new Set()
    try {
      mediaFiles = new Set(readdirSync(path.join(root, 'fixtures', 'media')))
    } catch {
      mediaFiles = new Set()
    }
    return selectPublicResearch(manifest, entries, revisions, mediaFiles)
  } catch {
    return emptyResearch(true)
  }
}
