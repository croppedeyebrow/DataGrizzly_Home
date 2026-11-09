/**
 * 개념: 관리자 글 목록.
 * 목적: 서버에 있는 글만 보여 주고, 비어 있으면 빈 상태를 숨기지 않는다.
 * 사용처: /admin/entries.
 */
import { useEffect, useState, type FormEvent } from 'react'
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

  async function onLogout() {
    await fetch('/api/admin/session', {
      method: 'DELETE',
      credentials: 'include',
      headers: { 'x-csrf-token': csrf },
    })
    clearOwnerSession()
    window.location.assign('/admin/login')
  }

  if (!csrf) {
    return (
      <main>
        <h1>게시물</h1>
        <p role="alert">로그인이 필요하다.</p>
        <p>
          <a href="/admin/login">소유자 로그인</a>
        </p>
      </main>
    )
  }

  return (
    <main>
      <h1>게시물</h1>
      <button type="button" onClick={onLogout}>
        로그아웃
      </button>
      <form onSubmit={onCreate}>
        <label htmlFor="entry-title">제목</label>
        <input id="entry-title" value={title} onChange={(event) => setTitle(event.target.value)} />
        <label htmlFor="entry-slug">슬러그</label>
        <input id="entry-slug" value={slug} onChange={(event) => setSlug(event.target.value)} />
        <label htmlFor="entry-kind">종류</label>
        <select id="entry-kind" value={kind} onChange={(event) => setKind(event.target.value)}>
          <option value="note">노트</option>
          <option value="paper">논문</option>
          <option value="news">소식</option>
          <option value="log">프로젝트 로그</option>
        </select>
        <button type="submit" disabled={pending}>
          작성하기
        </button>
      </form>
      {message ? <p role="alert">{message}</p> : null}
      {entries === null ? <p>목록을 불러오는 중이다.</p> : null}
      {entries && entries.length === 0 ? <p>글이 없다.</p> : null}
      {entries && entries.length > 0 ? (
        <ul>
          {entries.map((entry) => (
            <li key={entry.id}>
              <a href={`/admin/entries/${entry.id}/edit`}>{entry.title}</a>
              <span>
                {' '}
                {entry.kind} · {entry.published ? '공개 수정본 있음' : '초안'} · {entry.updatedAt}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
    </main>
  )
}
