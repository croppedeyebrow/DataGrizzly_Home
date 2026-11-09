/**
 * 개념: 관리자 화면의 자수정 글래스 틀.
 * 목적: 이미 있는 게시물 화면이 같은 패널과 메뉴를 쓰게 한다.
 * 사용처: 게시물 목록과 블록 편집.
 */
import type { ReactNode } from 'react'
import { clearOwnerSession, readCsrf } from './session'

/**
 * 개념: 왼쪽 메뉴와 본문.
 * 목적: 아직 없는 관리 메뉴를 링크로 만들지 않고, 게시물만 현재 화면으로 둔다.
 * 사용처: EntryList, Editor.
 */
export function AdminFrame({ title, children }: { title: string; children: ReactNode }) {
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
      <aside className="dg-panel flex h-fit flex-col gap-4 p-4">
        <a className="flex items-center gap-3 font-bold text-ink no-underline" href="/admin/entries">
          <span className="dg-icon-tile size-9 text-sm" aria-hidden="true">D</span>
          <span>
            DataGrizzly
            <br />
            관리자
          </span>
        </a>
        <a className="dg-nav-active rounded-[var(--dg-radius-control)] px-3 py-2 no-underline" href="/admin/entries">
          게시물
        </a>
        {csrf ? (
          <>
            <p className="mb-0 text-sm text-quiet">소유자 계정으로 로그인함</p>
            <button className="dg-btn-ghost mt-4" type="button" onClick={onLogout}>
              로그아웃
            </button>
          </>
        ) : (
          <a className="dg-btn-ghost" href="/admin/login">
            로그인
          </a>
        )}
      </aside>
      <div>
        <h1 className="mb-4 text-3xl">{title}</h1>
        {children}
      </div>
    </div>
  )
}
