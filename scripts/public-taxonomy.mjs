/**
 * 개념: 공개 주제와 시리즈를 고른다.
 * 목적: 공개 목록에 있는 글만 분류 페이지로 만들고, 빈 분류와 잘못된 slug를 페이지로 내지 않는다.
 * 사용처: apps/web/src/pages/topics/[slug].astro, apps/web/src/pages/series/[slug].astro, scripts/public-index.mjs, scripts/check-m1-seo.mjs
 * schema 1 본문에는 주제 필드가 없다. 분류는 manifest.taxonomies에 있을 때만 읽는다.
 */
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const SLUG = /^[a-z0-9]+(?:-[a-z0-9]+)*$/

function readJson(filePath) {
  return JSON.parse(readFileSync(filePath, 'utf8'))
}

function publicSlug(value) {
  return typeof value === 'string' && SLUG.test(value) ? value : ''
}

function pathFor(role, slug) {
  if (role === 'project') return `/projects/${slug}`
  if (role === 'log') return `/logs/${slug}`
  if (role === 'note') return `/notes/${slug}`
  if (role === 'paper' || role === 'news') return `/research/${slug}`
  return ''
}

function emptyTaxonomy(loadError) {
  return { loadError, topics: [], series: [], rejected: [] }
}

function asEntry(entry) {
  if (!entry || entry.example === false || !SLUG.test(entry.slug || '') || !entry.entryId) return null
  if (typeof entry.path !== 'string' || !entry.path.startsWith('/')) return null
  return entry
}

function takePage(kind, record, pages, slugs, orderedEntries) {
  const slug = typeof record?.slug === 'string' ? record.slug : ''
  if (!SLUG.test(slug)) return { kind, slug, reason: 'invalid-slug' }
  if (record.example !== true) return { kind, slug, reason: 'not-example' }
  if (typeof record.title !== 'string' || record.title.trim() === '') return { kind, slug, reason: 'invalid-title' }
  if (slugs.has(slug)) return { kind, slug, reason: 'duplicate' }
  if (orderedEntries.length === 0) return { kind, slug, reason: 'empty' }
  slugs.add(slug)
  pages.push({
    kind,
    slug,
    title: record.title.trim(),
    entries: orderedEntries,
  })
  return null
}

export function classifySlug(kind, slug, selected) {
  if (!SLUG.test(slug || '')) return 'invalid-slug'
  const pages = kind === 'series' ? selected?.series ?? [] : selected?.topics ?? []
  if (pages.some((page) => page.slug === slug)) return 'page'
  const rejected = (selected?.rejected ?? []).find((item) => item.kind === kind && item.slug === slug)
  if (rejected) return rejected.reason
  return 'unknown'
}

export function selectPublicTaxonomy(manifest, entries) {
  if (!manifest || manifest.dataset !== 'public-example') {
    return { loadError: false, topics: [], series: [], rejected: [{ kind: 'taxonomy', slug: '', reason: 'foreign-dataset' }] }
  }
  const publicById = new Map()
  for (const entry of entries ?? []) {
    const accepted = asEntry(entry)
    if (accepted) publicById.set(accepted.entryId, accepted)
  }
  const topics = []
  const series = []
  const rejected = []
  const topicSlugs = new Set()
  const seriesSlugs = new Set()
  for (const topic of manifest.taxonomies?.topics ?? []) {
    const seen = new Set()
    const ordered = []
    for (const entryId of topic?.entryIds ?? []) {
      const entry = publicById.get(entryId)
      if (!entry || seen.has(entry.entryId)) continue
      seen.add(entry.entryId)
      ordered.push({ entryId: entry.entryId, slug: entry.slug, title: entry.title, path: entry.path })
    }
    const reason = takePage('topic', topic, topics, topicSlugs, ordered)
    if (reason) rejected.push(reason)
  }
  for (const item of manifest.taxonomies?.series ?? []) {
    const ranked = (item?.items ?? [])
      .filter((entry) => Number.isInteger(entry?.position) && entry.position >= 0 && entry.position <= 10000)
      .map((entry) => ({ position: entry.position, entry: publicById.get(entry.entryId) }))
      .filter((entry) => entry.entry)
      .sort((a, b) => a.position - b.position || (a.entry.entryId < b.entry.entryId ? -1 : a.entry.entryId > b.entry.entryId ? 1 : 0))
    const seen = new Set()
    const ordered = []
    for (const entry of ranked) {
      if (seen.has(entry.entry.entryId)) continue
      seen.add(entry.entry.entryId)
      ordered.push({
        entryId: entry.entry.entryId,
        slug: entry.entry.slug,
        title: entry.entry.title,
        path: entry.entry.path,
        position: entry.position,
      })
    }
    const reason = takePage('series', item, series, seriesSlugs, ordered)
    if (reason) rejected.push(reason)
  }
  return { loadError: false, topics, series, rejected }
}

export function loadPublicTaxonomy(contentRoot = path.join(root, 'fixtures', 'content')) {
  /* 상대 경로는 실행 중인 폴더 기준이다. 페이지는 apps/web에서 ../../fixtures/content를 넘긴다. */
  const resolved = path.isAbsolute(contentRoot) ? contentRoot : path.resolve(process.cwd(), contentRoot)
  try {
    const manifest = readJson(path.join(resolved, 'manifest.json'))
    const entries = []
    for (const item of manifest.public ?? []) {
      const entry = readJson(path.join(resolved, item.entry))
      const revision = readJson(path.join(resolved, item.revision))
      const slug = publicSlug(entry.slug)
      if (entry.example !== true || revision.example !== true || entry.publishedRevisionId !== revision.revisionId || !slug) continue
      const route = pathFor(item.role, slug)
      if (!route) continue
      entries.push({
        entryId: entry.entryId,
        slug,
        title: revision.title,
        path: route,
        example: true,
      })
    }
    return selectPublicTaxonomy(manifest, entries)
  } catch {
    return emptyTaxonomy(true)
  }
}
