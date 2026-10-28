/**
 * 개념: 공개 프로젝트와 과정 기록을 페이지 경로로 고른다.
 * 목적: 공개 목록에 없거나 slug가 없는 글은 주소를 만들지 않는다. 이전과 다음은 공개 과정만으로 계산한다.
 * 사용처: apps/web/src/pages/projects, apps/web/src/pages/logs, scripts/check-m1-projects.mjs
 * schema 1에는 비공개 플래그와 GitHub 주소, 주제 필드가 없다. 공개 목록에 없는 글을 뺀다.
 */
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')

const stageLabel = {
  planning: '기획',
  implementation: '구현',
  verification: '검증',
}

function readJson(filePath) {
  return JSON.parse(readFileSync(filePath, 'utf8'))
}

function paragraphAfter(revision, anchor) {
  const blocks = revision.blocks ?? []
  const index = blocks.findIndex((block) => block.anchor === anchor)
  if (index < 0) return ''
  const next = blocks[index + 1]
  return next?.type === 'paragraph' ? next.text : ''
}

function firstImage(revision) {
  return (revision.blocks ?? []).find((block) => block.type === 'image') ?? null
}

function publicSlug(value) {
  return typeof value === 'string' && /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(value) ? value : ''
}

export function emptyProjects(loadError) {
  return { loadError, projects: [], logs: [] }
}

function acceptedItem(item, entries, revisions) {
  const entry = entries[item.entry]
  const revision = revisions[item.revision]
  if (!entry || !revision) return null
  if (entry.example !== true || revision.example !== true) return null
  if (entry.publishedRevisionId !== revision.revisionId) return null
  if (entry.type !== item.role || revision.metadata?.type !== item.role) return null
  const slug = publicSlug(entry.slug)
  if (!slug) return null
  return { role: item.role, entry, revision, slug }
}

function relatedLinks(revision, byEntryId) {
  return (revision.blocks ?? [])
    .filter((block) => block.type === 'related' && byEntryId.has(block.entryId))
    .map((block) => {
      const target = byEntryId.get(block.entryId)
      const href = target.role === 'log'
        ? `/logs/${target.slug}`
        : target.role === 'project'
          ? `/projects/${target.slug}`
          : null
      return { title: target.revision.title, href, kind: target.role }
    })
}

function compareLogs(a, b) {
  if (a.displayOrder !== b.displayOrder) return a.displayOrder - b.displayOrder
  if (a.entryId < b.entryId) return -1
  if (a.entryId > b.entryId) return 1
  return 0
}

export function selectPublicProjects(manifest, entries, revisions) {
  if (!manifest || manifest.dataset !== 'public-example' || !Array.isArray(manifest.public)) {
    return emptyProjects(false)
  }

  const items = manifest.public
    .map((item) => acceptedItem(item, entries, revisions))
    .filter(Boolean)
  const byEntryId = new Map(items.map((item) => [item.entry.entryId, item]))
  const projects = items
    .filter((item) => item.role === 'project')
    .map((item) => {
      const image = firstImage(item.revision)
      const logs = items
        .filter((log) => log.role === 'log' && log.revision.metadata?.projectId === item.entry.entryId)
        .filter((log) => typeof log.revision.metadata?.displayOrder === 'number')
        .map((log) => ({
          slug: log.slug,
          title: log.revision.title,
          entryId: log.entry.entryId,
          displayOrder: log.revision.metadata.displayOrder,
          createdAt: log.revision.createdAt,
          stage: stageLabel[log.revision.metadata.stage] ?? log.revision.metadata.stage,
          paragraphs: (log.revision.blocks ?? []).filter((block) => block.type === 'paragraph').map((block) => block.text),
          callout: (log.revision.blocks ?? []).find((block) => block.type === 'callout')?.text ?? '',
          projectSlug: item.slug,
          projectTitle: item.revision.title,
          related: relatedLinks(log.revision, byEntryId),
        }))
        .sort(compareLogs)
      const uniqueLogs = []
      const projectSlugs = new Set()
      for (const log of logs) {
        if (projectSlugs.has(log.slug)) continue
        projectSlugs.add(log.slug)
        uniqueLogs.push(log)
      }
      const orderedLogs = uniqueLogs.map((log, index, list) => ({
        ...log,
        previous: index > 0 ? { slug: list[index - 1].slug, title: list[index - 1].title } : null,
        next: index < list.length - 1 ? { slug: list[index + 1].slug, title: list[index + 1].title } : null,
      }))

      return {
        slug: item.slug,
        title: item.revision.title,
        summary: paragraphAfter(item.revision, 'problem'),
        status: paragraphAfter(item.revision, 'status'),
        updatedAt: item.entry.updatedAt,
        callout: (item.revision.blocks ?? []).find((block) => block.type === 'callout')?.text ?? '',
        problem: paragraphAfter(item.revision, 'problem'),
        goal: paragraphAfter(item.revision, 'goal'),
        scope: paragraphAfter(item.revision, 'scope'),
        choice: paragraphAfter(item.revision, 'choice'),
        judgment: paragraphAfter(item.revision, 'judgment'),
        result: paragraphAfter(item.revision, 'result'),
        limit: paragraphAfter(item.revision, 'limit'),
        imageAlt: image?.alt ?? '',
        imageName: image ? new URL(image.url).pathname.split('/').pop() : '',
        topic: null,
        github: null,
        logs: orderedLogs,
        related: relatedLinks(item.revision, byEntryId),
      }
    })
    .sort((a, b) => {
      if (a.updatedAt !== b.updatedAt) return a.updatedAt < b.updatedAt ? 1 : -1
      if (a.slug < b.slug) return -1
      if (a.slug > b.slug) return 1
      return 0
    })

  const seen = new Set()
  const logs = []
  for (const project of projects) {
    const kept = []
    for (const log of project.logs) {
      if (seen.has(log.slug)) continue
      seen.add(log.slug)
      kept.push(log)
    }
    project.logs = kept.map((log, index, list) => ({
      ...log,
      previous: index > 0 ? { slug: list[index - 1].slug, title: list[index - 1].title } : null,
      next: index < list.length - 1 ? { slug: list[index + 1].slug, title: list[index + 1].title } : null,
    }))
    logs.push(...project.logs)
  }

  return { loadError: false, projects, logs }
}

export function loadPublicProjects(contentRoot = path.join(root, 'fixtures', 'content')) {
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
    return selectPublicProjects(manifest, entries, revisions)
  } catch {
    return emptyProjects(true)
  }
}
