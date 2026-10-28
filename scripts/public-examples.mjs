/**
 * 개념: 공개 예시 목록을 홈이 읽을 수 있는 값으로 고른다.
 * 목적: 경계 fixture와 예시가 아닌 글을 홈에 섞지 않는다. 연락 링크는 fixture에 없으므로 비운다.
 * 사용처: apps/web/src/pages/index.astro, scripts/check-m1-layout.mjs
 * slug는 프로젝트 설명과 과정 상세로 이어지는 공개 경로에 쓴다.
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

export function emptyHome(loadError) {
  return {
    loadError,
    project: null,
    logs: [],
    topics: [],
    contact: null,
  }
}

export function selectPublicHome(manifest, revisions, entriesByRevision = {}) {
  if (!manifest || manifest.dataset !== 'public-example' || !Array.isArray(manifest.public)) {
    return emptyHome(false)
  }
  const projectItem = manifest.public.find((item) => item.role === 'project')
  const projectRevision = projectItem ? revisions[projectItem.revision] : null
  const project = projectRevision?.example === true
    ? {
        title: projectRevision.title,
        slug: publicSlug(entriesByRevision[projectItem.revision]?.slug),
        summary: paragraphAfter(projectRevision, 'problem'),
        imageAlt: firstImage(projectRevision)?.alt ?? '',
        imageName: firstImage(projectRevision) ? new URL(firstImage(projectRevision).url).pathname.split('/').pop() : '',
      }
    : null

  const logs = manifest.public
    .filter((item) => item.role === 'log')
    .map((item) => ({ revision: revisions[item.revision], entry: entriesByRevision[item.revision] }))
    .filter((item) => item.revision?.example === true && item.revision.metadata?.projectId === manifest.projectId)
    .sort((a, b) => a.revision.metadata.displayOrder - b.revision.metadata.displayOrder)
    .map((item) => ({
      title: item.revision.title,
      slug: publicSlug(item.entry?.slug),
      stage: stageLabel[item.revision.metadata.stage] ?? item.revision.metadata.stage,
      createdAt: item.revision.createdAt,
      displayOrder: item.revision.metadata.displayOrder,
    }))

  const topics = manifest.public
    .filter((item) => item.role === 'note' || item.role === 'paper' || item.role === 'news')
    .map((item) => {
      const revision = revisions[item.revision]
      if (revision?.example !== true) return null
      return { title: revision.title, kind: item.role }
    })
    .filter(Boolean)

  return { loadError: false, project, logs, topics, contact: null }
}

export function loadPublicHome(contentRoot = path.join(root, 'fixtures', 'content')) {
  /* 상대 경로는 실행 중인 폴더 기준이다. 홈은 apps/web에서 ../../fixtures/content를 넘긴다. */
  const resolved = path.isAbsolute(contentRoot) ? contentRoot : path.resolve(process.cwd(), contentRoot)
  try {
    const manifest = readJson(path.join(resolved, 'manifest.json'))
    const revisions = {}
    const entriesByRevision = {}
    for (const item of manifest.public ?? []) {
      revisions[item.revision] = readJson(path.join(resolved, item.revision))
      entriesByRevision[item.revision] = readJson(path.join(resolved, item.entry))
    }
    return selectPublicHome(manifest, revisions, entriesByRevision)
  } catch {
    return emptyHome(true)
  }
}
