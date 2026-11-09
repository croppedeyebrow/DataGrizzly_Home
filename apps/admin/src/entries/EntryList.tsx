/**
 * 개념: 관리자 글 목록.
 * 목적: 서버에 있는 글만 보여 주고, 비어 있으면 빈 상태를 숨기지 않는다.
 * 사용처: /admin/entries.
 */
import { useEffect, useState, type FormEvent } from 'react'
import { AdminFrame } from '../AdminFrame'
import { clearOwnerSession, readCsrf } from '../session'

type EntryRow = {
  id: string
  slug: string
  title: string
  kind: string
  version: number
  updatedAt: string
  published: boolean
}

/**
 * 개념: 글 목록과 새 글 폼.
 * 목적: 예시 제목을 넣지 않고 API 목록을 그대로 보여 준다.
 * 사용처: App의 entries 경로.
 */
export default function EntryList() {
  const csrf = readCsrf()
  const [entries, setEntries] = useState<EntryRow[] | null>(null)
  const [message, setMessage] = useState('')
  const [slug, setSlug] = useState('')
  const [title, setTitle] = useState('')
  const [kind, setKind] = useState('note')
  const [pending, setPending] = useState(false)

  useEffect(() => {
    if (!csrf) return
    let cancelled = false
    async function load() {
      try {
        const response = await fetch('/api/admin/entries', {
          credentials: 'include',
          headers: { 'x-csrf-token': csrf },
        })
        if (response.status === 401) {
          clearOwnerSession()
          window.location.assign('/admin/login')
          return
        }
        if (!response.ok) {
          if (!cancelled) setMessage('목록을 불러오지 못했다.')
          return
        }
        const body = (await response.json()) as { entries?: EntryRow[] }
        if (!cancelled) setEntries(body.entries ?? [])
      } catch {
        if (!cancelled) setMessage('서버에 연결하지 못했다.')
      }
    }
    void load()
    return () => {
      cancelled = true
    }
  }, [csrf])

  async function onCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setPending(true)
    setMessage('')
    try {
      const response = await fetch('/api/admin/entries', {
        method: 'POST',
        credentials: 'include',
        headers: { 'content-type': 'application/json', 'x-csrf-token': csrf },
        body: JSON.stringify({ slug, title, kind }),
      })
      if (response.status === 422) {
        setMessage('슬러그, 제목, 종류를 다시 확인해 달라.')
        return
      }
      if (response.status === 409) {
        setMessage('같은 슬러그의 글이 이미 있다.')
        return
      }
      if (!response.ok) {
        setMessage('글을 만들지 못했다.')
        return
      }
      const body = (await response.json()) as { id?: string }
      if (!body.id) {
        setMessage('글을 만들지 못했다.')
        return
      }
      window.location.assign(`/admin/entries/${body.id}/edit`)
    } catch {
      setMessage('서버에 연결하지 못했다. 입력한 내용은 이 화면에 남아 있다.')
    } finally {
      setPending(false)
    }
  }

  if (!csrf) {
    return (
      <AdminFrame title="게시물 관리">
        <p className="dg-item px-4 py-3" role="alert">로그인이 필요하다.</p>
        <p>
          <a className="dg-btn" href="/admin/login">소유자 로그인</a>
        </p>
      </AdminFrame>
    )
  }

  return (
    <AdminFrame title="게시물 관리">
      <p className="dg-label mb-4">제목 · 종류 · 초안/공개 · 수정일</p>
      <form className="dg-panel mb-4 grid gap-3 p-5" onSubmit={onCreate}>
        <label htmlFor="entry-title">제목</label>
        <input className="dg-input" id="entry-title" value={title} onChange={(event) => setTitle(event.target.value)} />
        <label htmlFor="entry-slug">슬러그</label>
        <input className="dg-input" id="entry-slug" value={slug} onChange={(event) => setSlug(event.target.value)} />
        <label htmlFor="entry-kind">종류</label>
        <select className="dg-input" id="entry-kind" value={kind} onChange={(event) => setKind(event.target.value)}>
          <option value="note">노트</option>
          <option value="paper">논문</option>
          <option value="news">소식</option>
          <option value="log">프로젝트 로그</option>
        </select>
        <button className="dg-btn" type="submit" disabled={pending}>
          새 항목
        </button>
      </form>
      {message ? <p className="dg-item px-4 py-3" role="alert">{message}</p> : null}
      {entries === null && !message ? <p>목록을 불러오는 중이다.</p> : null}
      {entries && entries.length === 0 ? <p className="dg-item px-4 py-3">글이 없다.</p> : null}
      {entries && entries.length > 0 ? (
        <ul className="m-0 grid list-none gap-3 p-0">
          {entries.map((entry) => (
            <li key={entry.id} className="dg-item flex flex-wrap items-center justify-between gap-3 px-4 py-3">
              <a href={`/admin/entries/${entry.id}/edit`}>{entry.title}</a>
              <span className="dg-chip">
                {{ note: '노트', paper: '논문 리뷰', news: '뉴스 정리', log: '과정 기록' }[entry.kind] ?? entry.kind}
                {' · '}
                {entry.published ? '공개' : '초안'}
                {' · '}
                {entry.updatedAt}
                {' · 편집'}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
    </AdminFrame>
  )
}
