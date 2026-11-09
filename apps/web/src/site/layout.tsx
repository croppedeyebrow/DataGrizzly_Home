/**
 * 개념: 공개 웹의 공통 틀.
 * 목적: 건너뛰기 링크, 헤더, 현재 메뉴, 본문 하나, 푸터를 페이지가 반복하지 않게 한다.
 * 사용처: apps/web/src/site/pages.tsx
 */
import type { ReactNode } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { buildMeta, siteOrigin } from '../external'

export type Current = '/' | '/about' | '/projects' | '/notes' | '/research' | '/explore' | null

const items: { href: Current & string; label: string }[] = [
  { href: '/', label: '홈' },
  { href: '/about', label: '소개' },
  { href: '/projects', label: '프로젝트' },
  { href: '/notes', label: '노트' },
  { href: '/research', label: '리서치' },
  { href: '/explore', label: '탐색' },
]

export type FrameProps = {
  title: string
  current: Current
  canonical?: string
  description?: string
  date?: string
  dateKind?: 'published' | 'modified'
  includeCanonical?: boolean
  noindex?: boolean
  children: ReactNode
}

const focus =
  'focus-visible:outline-none focus-visible:shadow-[0_0_0_3px_var(--color-focus)]'

export function frameMeta(props: FrameProps) {
  const includeCanonical = props.includeCanonical !== false
  const canonical = props.canonical ?? props.current ?? '/'
  const description = props.description ?? '공개 예시 글을 정적 페이지로 보여 준다. API는 아직 연결하지 않았다.'
  return buildMeta({
    title: props.title,
    description,
    path: includeCanonical ? canonical : '',
    date: props.date ?? '',
    siteOrigin,
  })
}

export function Frame(props: FrameProps) {
  return (
    <>
      <a className={`dg-panel absolute left-4 top-4 -translate-y-[150%] px-3 py-2 text-ink focus:translate-y-0 ${focus}`} href="#main">
        본문으로 건너뛰기
      </a>
      <header className="mx-auto w-[min(calc(100%-2rem),var(--page-width))] pt-4">
        <div className="dg-panel flex flex-col gap-3 px-4 py-3 min-[48rem]:grid min-[48rem]:grid-cols-[1fr_auto_1fr] min-[48rem]:items-center">
          <a className={`flex items-center gap-3 font-bold text-ink no-underline ${focus}`} href="/">
            <span className="dg-icon-tile size-9 text-sm" aria-hidden="true">D</span>
            DataGrizzly
          </a>
          <details id="site-menu" className="group min-[48rem]:mt-0 min-[48rem]:justify-self-center" open>
            <summary className={`w-fit cursor-pointer min-[48rem]:hidden ${focus}`}>메뉴</summary>
            <nav className="site-nav hidden group-open:block min-[48rem]:block" aria-label="공개 웹">
              <ul className="m-0 flex list-none flex-wrap gap-2 p-0">
                {items.map((item) => (
                  <li key={item.href}>
                    <a
                      className={`rounded-[var(--dg-radius-control)] px-3 py-2 text-ink no-underline ${focus}`}
                      href={item.href}
                      aria-current={item.href === props.current ? 'page' : undefined}
                    >
                      {item.label}
                    </a>
                  </li>
                ))}
              </ul>
            </nav>
          </details>
          <span className="hidden min-[48rem]:block" aria-hidden="true" />
        </div>
      </header>
      <main id="main" className="mx-auto w-[min(calc(100%-2rem),var(--page-width))] py-8">
        {props.children}
      </main>
      <footer className="mx-auto flex w-[min(calc(100%-2rem),var(--page-width))] items-center justify-between gap-4 px-1 pb-8 text-sm text-quiet">
        <p className="m-0">© DataGrizzly</p>
        <p className="m-0">
          GitHub
          <span aria-hidden="true"> · </span>
          <a href="/rss.xml">RSS</a>
        </p>
      </footer>
    </>
  )
}

export function documentHtml(props: FrameProps, css: string | null) {
  const meta = frameMeta(props)
  const includeCanonical = props.includeCanonical !== false
  const dateKind = props.dateKind ?? 'published'
  const body = renderToStaticMarkup(<Frame {...props} />)
  const style = css === null ? '' : `<style>${css}</style>`
  const script = css === null
    ? '<script type="module" src="/src/boot.ts"></script>'
    : '<script type="module" src="/menu.js"></script>'
  const canonical = includeCanonical && meta.canonical ? `<link rel="canonical" href="${meta.canonical}" />` : ''
  const robots = props.noindex ? '<meta name="robots" content="noindex" />' : ''
  const description = meta.description ? `<meta name="description" content="${escapeAttr(meta.description)}" />` : ''
  const ogDescription = meta.og.description ? `<meta property="og:description" content="${escapeAttr(meta.og.description)}" />` : ''
  const ogUrl = includeCanonical && meta.canonical ? `<meta property="og:url" content="${escapeAttr(meta.og.url)}" />` : ''
  const time = meta.date
    ? `<meta property="${dateKind === 'modified' ? 'article:modified_time' : 'article:published_time'}" content="${escapeAttr(meta.date)}" />`
    : ''
  return `<!doctype html>
<html lang="ko">
  <head>
    <meta charset="utf-8" />
    <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
    <meta name="viewport" content="width=device-width" />
    <title>${escapeText(meta.title)}</title>
    ${description}
    <meta name="author" content="${escapeAttr(meta.author)}" />
    ${robots}
    ${canonical}
    <meta property="og:title" content="${escapeAttr(meta.og.title)}" />
    ${ogDescription}
    <meta property="og:type" content="${escapeAttr(meta.og.type)}" />
    ${ogUrl}
    ${time}
    ${style}
  </head>
  <body>
    ${body}
    ${script}
  </body>
</html>`
}

function escapeText(value: string) {
  return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;')
}

function escapeAttr(value: string) {
  return escapeText(value).replaceAll('"', '&quot;')
}
