/**
 * 개념: 지금 공개된 주소 목록.
 * 목적: sitemap과 RSS가 공개 snapshot만 담고, 관리 화면·초안·필터 조합을 넣지 않게 한다.
 * 사용처: apps/web/src/pages/sitemap.xml.ts, apps/web/src/pages/rss.xml.ts, scripts/check-m1-seo.mjs
 */
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadPublicProjects } from './public-projects.mjs'
import { loadPublicResearch } from './public-research.mjs'
import { loadPublicTaxonomy } from './public-taxonomy.mjs'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')

const fixedPages = [
  { path: '/', title: 'DataGrizzly', kind: 'page' },
  { path: '/about', title: '소개 · DataGrizzly', kind: 'page' },
  { path: '/projects', title: '프로젝트 · DataGrizzly', kind: 'page' },
  { path: '/notes', title: '노트 · DataGrizzly', kind: 'page' },
  { path: '/research', title: '리서치 · DataGrizzly', kind: 'page' },
]

export function collectPublicRecords(projects, research, taxonomy) {
  const records = [...fixedPages]
  if (!projects?.loadError) {
    for (const project of projects.projects ?? []) {
      records.push({ path: `/projects/${project.slug}`, title: project.title, kind: 'entry', updatedAt: project.updatedAt })
      records.push({ path: `/projects/${project.slug}/logs`, title: `${project.title} 과정`, kind: 'page' })
    }
    for (const log of projects.logs ?? []) {
      records.push({ path: `/logs/${log.slug}`, title: log.title, kind: 'entry', updatedAt: log.createdAt })
    }
  }
  if (!research?.loadError) {
    for (const note of research.notes ?? []) {
      records.push({ path: `/notes/${note.slug}`, title: note.title, kind: 'entry', updatedAt: note.updatedAt })
    }
    for (const article of research.research ?? []) {
      records.push({ path: `/research/${article.slug}`, title: article.title, kind: 'entry', updatedAt: article.updatedAt })
    }
  }
  for (const topic of taxonomy?.topics ?? []) {
    records.push({ path: `/topics/${topic.slug}`, title: topic.title, kind: 'page' })
  }
  for (const series of taxonomy?.series ?? []) {
    records.push({ path: `/series/${series.slug}`, title: series.title, kind: 'page' })
  }
  return records
}

export function loadPublicIndex(contentRoot = path.join(root, 'fixtures', 'content')) {
  /* 상대 경로는 실행 중인 폴더 기준이다. 페이지는 apps/web에서 ../../fixtures/content를 넘긴다. */
  const resolved = path.isAbsolute(contentRoot) ? contentRoot : path.resolve(process.cwd(), contentRoot)
  try {
    const projects = loadPublicProjects(resolved)
    const research = loadPublicResearch(resolved)
    const taxonomy = loadPublicTaxonomy(resolved)
    return {
      loadError: Boolean(projects.loadError || research.loadError || taxonomy.loadError),
      records: collectPublicRecords(projects, research, taxonomy),
    }
  } catch {
    return { loadError: true, records: [...fixedPages] }
  }
}
