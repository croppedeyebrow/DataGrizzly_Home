/**
 * 개념: 관리자 화면의 자수정 글래스 틀.
 * 목적: 게시물, 프로젝트, 분류, 미디어가 같은 패널과 메뉴를 쓰게 한다.
 * 사용처: 관리 화면 전체.
 */
import type { ReactNode } from 'react'
import { clearOwnerSession, readCsrf } from './session'

type AdminSection = 'entries' | 'projects' | 'taxonomies' | 'media'

const links: { id: AdminSection; href: string; label: string }[] = [
  { id: 'entries', href: '/admin/entries', label: '게시물' },
  { id: 'projects', href: '/admin/projects', label: '프로젝트' },
  { id: 'taxonomies', href: '/admin/taxonomies', label: '분류 · 시리즈' },
  { id: 'media', href: '/admin/media', label: '미디어' },
]

/**
 * 개념: 왼쪽 메뉴와 본문.
 * 목적: 이번 태스크에서 동작하는 메뉴만 이동하게 하고, 아직 없는 화면은 글자로만 둔다.
 * 사용처: 게시물, 프로젝트, 분류, 미디어, 블록 편집.
 */
export function AdminFrame({
  title,
  label,
  action,
  current = 'entries',
  children,
}: {
  title: string
  label?: string
  action?: ReactNode
  current?: AdminSection
  children: ReactNode
}) {
  const csrf = readCsrf()

  async function onLogout() {
    await fetch('/api/admin/session', {
      method: 'DELETE',
      credentials: 'include',
      headers: { 'x-csrf-token': csrf },
    })
    clearOwnerSession()
    window.location.assign('/admin/login')
  }

  return (
    <div className="mx-auto grid w-[min(calc(100%-1.5rem),76rem)] gap-4 py-6 min-[64rem]:grid-cols-[15rem_1fr]">
      <aside className="dg-panel flex h-fit flex-col gap-2 p-4">
        <a className="mb-2 flex items-center gap-3 font-bold text-ink no-underline" href="/admin/entries">
          <span className="dg-icon-tile size-9 text-sm" aria-hidden="true">D</span>
          <span>
            DataGrizzly
            <br />
            관리자
          </span>
        </a>
        <span className="px-3 py-2 text-sm text-quiet">관리 홈</span>
        {links.map((item) => (
          <a
            key={item.id}
            className={
              item.id === current
                ? 'dg-nav-active rounded-[var(--dg-radius-control)] px-3 py-2 no-underline'
                : 'rounded-[var(--dg-radius-control)] px-3 py-2 text-ink no-underline'
            }
            href={item.href}
            aria-current={item.id === current ? 'page' : undefined}
          >
            {item.label}
          </a>
        ))}
        <span className="px-3 py-2 text-sm text-quiet">발행 작업</span>
        <span className="px-3 py-2 text-sm text-quiet">검색 상태</span>
        <span className="px-3 py-2 text-sm text-quiet">사이트 설정</span>
        {csrf ? (
          <>
            <p className="mb-0 mt-4 text-sm text-quiet">소유자 계정으로 로그인함</p>
            <button className="dg-btn-ghost mt-2" type="button" onClick={onLogout}>
              로그아웃
            </button>
          </>
        ) : (
          <a className="dg-btn-ghost mt-4" href="/admin/login">
            로그인
          </a>
        )}
      </aside>
      <div>
        <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
          <div>
            <h1 className="mb-1 text-3xl">{title}</h1>
            {label ? <p className="dg-label mb-0">{label}</p> : null}
          </div>
          {action}
        </div>
        {children}
      </div>
    </div>
  )
}
