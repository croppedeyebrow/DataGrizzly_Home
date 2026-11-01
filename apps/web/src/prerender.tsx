/**
 * 개념: 공개 경로를 정적 HTML과 수집 문서로 만든다.
 * 목적: 개발 서버와 빌드가 같은 페이지 집합을 쓰게 한다.
 * 사용처: apps/web/vite.config.ts, apps/web/build.mjs
 */
import { fileURLToPath } from 'node:url'
import type { ReactElement } from 'react'
import {
  buildRss,
  buildSitemap,
  loadPublicHome,
  loadPublicIndex,
  loadPublicProjects,
  loadPublicResearch,
  loadPublicTaxonomy,
  siteOrigin,
} from './external'
import { documentHtml, type FrameProps } from './site/layout'
import {
  AboutPage,
  HomePage,
  LogPage,
  MissingPage,
  NoteListPage,
  NotePage,
  ProjectListPage,
  ProjectLogsPage,
  ProjectPage,
  ResearchListPage,
  ResearchPage,
  SeriesPage,
  TopicPage,
} from './site/pages'

const contentRoot = fileURLToPath(new URL('../../../fixtures/content/', import.meta.url))

export type SiteFile = { path: string; body: string; kind: 'html' | 'xml' }

type Route = { path: string; element: ReactElement }

function routes(): Route[] {
  const home = loadPublicHome(contentRoot)
  const projects = loadPublicProjects(contentRoot)
  const research = loadPublicResearch(contentRoot)
  const taxonomy = loadPublicTaxonomy(contentRoot)
  const pages: Route[] = [
    { path: '/', element: HomePage({ home }) },
    { path: '/about', element: AboutPage() },
    { path: '/projects', element: ProjectListPage({ projects: projects.projects, loadError: projects.loadError }) },
    { path: '/notes', element: NoteListPage({ notes: research.notes, loadError: research.loadError }) },
    { path: '/research', element: ResearchListPage({ articles: research.research, loadError: research.loadError }) },
    { path: '/404', element: MissingPage() },
  ]
  for (const project of projects.projects) {
    pages.push({ path: `/projects/${project.slug}`, element: ProjectPage({ project }) })
    pages.push({ path: `/projects/${project.slug}/logs`, element: ProjectLogsPage({ project }) })
  }
  for (const log of projects.logs) pages.push({ path: `/logs/${log.slug}`, element: LogPage({ log }) })
  for (const article of research.notes) pages.push({ path: `/notes/${article.slug}`, element: NotePage({ article }) })
  for (const article of research.research) pages.push({ path: `/research/${article.slug}`, element: ResearchPage({ article }) })
  if (!taxonomy.loadError) {
    for (const topic of taxonomy.topics.filter((item) => item.entries.length > 0)) {
      pages.push({ path: `/topics/${topic.slug}`, element: TopicPage({ topic }) })
    }
    for (const series of taxonomy.series.filter((item) => item.entries.length > 0)) {
      pages.push({ path: `/series/${series.slug}`, element: SeriesPage({ series }) })
    }
  }
  return pages
}

function filePath(route: string) {
  if (route === '/404') return '404.html'
  if (route === '/') return 'index.html'
  return `${route.slice(1)}/index.html`
}

function htmlFor(element: ReactElement, css: string | null) {
  return documentHtml(element.props as FrameProps, css)
}

export function renderSite(css: string): SiteFile[] {
  const files: SiteFile[] = routes().map((route) => ({
    path: filePath(route.path),
    body: htmlFor(route.element, css),
    kind: 'html' as const,
  }))
  const index = loadPublicIndex(contentRoot)
  files.push({ path: 'sitemap.xml', body: buildSitemap(index.records, siteOrigin).xml, kind: 'xml' })
  files.push({ path: 'rss.xml', body: buildRss(index.records, siteOrigin).xml, kind: 'xml' })
  return files
}

export function renderRequest(url: string): SiteFile | null {
  const raw = url.split('?')[0] ?? '/'
  const normalized = raw.length > 1 ? raw.replace(/\/+$/, '') : raw
  if (normalized === '/sitemap.xml' || normalized === '/rss.xml') {
    const index = loadPublicIndex(contentRoot)
    const built = normalized === '/sitemap.xml' ? buildSitemap(index.records, siteOrigin) : buildRss(index.records, siteOrigin)
    return { path: normalized.slice(1), body: built.xml, kind: 'xml' }
  }
  const route = routes().find((item) => item.path === normalized) ?? routes().find((item) => item.path === '/404')
  if (!route) return null
  return { path: filePath(route.path), body: htmlFor(route.element, null), kind: 'html' }
}
