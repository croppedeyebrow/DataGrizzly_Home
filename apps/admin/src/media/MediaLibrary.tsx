/**
 * 개념: 미디어 관리 화면.
 * 목적: 허용된 파일만 올리고, 참조 중인 파일은 지우지 않는다.
 * 사용처: /admin/media.
 */
import { useEffect, useState, type DragEvent, type FormEvent } from 'react'
import { AdminFrame } from '../AdminFrame'
import { adminJson } from '../api'
import { readCsrf } from '../session'

type MediaItem = {
  id: string
  originalName: string
  mediaType: string
  bytes: number
  status: string
  public: boolean
  alt: string
  caption: string
  usageCount: number
}

type EntryOption = { id: string; title: string }

const limit = 8 * 1024 * 1024

function fileBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const value = String(reader.result ?? '')
      const comma = value.indexOf(',')
      resolve(comma >= 0 ? value.slice(comma + 1) : value)
    }
    reader.onerror = () => reject(new Error('read'))
    reader.readAsDataURL(file)
  })
}

/**
 * 개념: 업로드와 파일 카드.
 * 목적: 비공개 파일은 인증된 요청으로만 미리보고, 한 페이지에 8개만 둔다.
 * 사용처: App의 media 경로.
 */
export default function MediaLibrary() {
  const csrf = readCsrf()
  const [items, setItems] = useState<MediaItem[] | null>(null)
  const [page, setPage] = useState(1)
  const [total, setTotal] = useState(0)
  const [message, setMessage] = useState('')
  const [pending, setPending] = useState(false)
  const [selected, setSelected] = useState<MediaItem | null>(null)
  const [alt, setAlt] = useState('')
  const [caption, setCaption] = useState('')
  const [preview, setPreview] = useState('')
  const [entries, setEntries] = useState<EntryOption[]>([])
  const [entryId, setEntryId] = useState('')

  async function load(nextPage: number) {
    const result = await adminJson<{ media?: MediaItem[]; total?: number }>(`/api/admin/media?page=${nextPage}`)
    if (result.status === 401) return
    if (result.status !== 200 || !result.body?.media) {
      setMessage(result.status === 0 ? '서버에 연결하지 못했다.' : '목록을 불러오지 못했다.')
      return
    }
    setItems(result.body.media)
    setTotal(result.body.total ?? result.body.media.length)
    setPage(nextPage)
  }

  useEffect(() => {
    if (!csrf) return
    void load(1)
    void adminJson<{ entries?: EntryOption[] }>('/api/admin/entries').then((result) => {
      if (result.status === 200 && result.body?.entries) setEntries(result.body.entries)
    })
  }, [csrf])

  useEffect(() => {
    if (!selected || !selected.mediaType.startsWith('image/') || selected.mediaType === 'image/svg+xml' || selected.bytes > 524288) {
      setPreview('')
      return
    }
    let url = ''
    let cancelled = false
    async function loadPreview() {
      const response = await fetch(`/api/admin/media/${selected?.id}/file`, {
        credentials: 'include',
        headers: { 'x-csrf-token': csrf },
      })
      if (!response.ok || cancelled) return
      const blob = await response.blob()
      if (cancelled) return
      url = URL.createObjectURL(blob)
      setPreview(url)
    }
    void loadPreview()
    return () => {
      cancelled = true
      if (url) URL.revokeObjectURL(url)
    }
  }, [selected, csrf])

  async function sendFile(file: File) {
    if (file.size > limit) {
      setMessage('최대 8MB까지 올릴 수 있다.')
      return
    }
    setPending(true)
    setMessage('')
    try {
      const contentBase64 = await fileBase64(file)
      const result = await adminJson<{ field?: string }>('/api/admin/media', {
        method: 'POST',
        body: JSON.stringify({ filename: file.name, contentBase64 }),
      })
      if (result.status === 422) {
        const field = result.body?.field
        setMessage(field === 'bytes' ? '최대 8MB까지 올릴 수 있다.' : '허용된 형식이 아니다. PNG, JPG, WEBP, SVG, PDF만 올릴 수 있다.')
        return
      }
      if (result.status !== 201) {
        setMessage('파일을 올리지 못했다.')
        return
      }
      setMessage('파일을 올렸다. 지금은 비공개다.')
      await load(1)
    } catch {
      setMessage('파일을 읽지 못했다.')
    } finally {
      setPending(false)
    }
  }

  function onDrop(event: DragEvent<HTMLElement>) {
    event.preventDefault()
    const file = event.dataTransfer.files.item(0)
    if (file) void sendFile(file)
  }

  async function onVisibility(item: MediaItem) {
    setMessage('')
    const result = await adminJson(`/api/admin/media/${item.id}/visibility`, {
      method: 'POST',
      body: JSON.stringify({ public: !item.public }),
    })
    if (result.status !== 200) {
      setMessage('공개 상태를 바꾸지 못했다.')
      return
    }
    await load(page)
  }

  async function onDelete(item: MediaItem) {
    setMessage('')
    const result = await adminJson<{ error?: string }>(`/api/admin/media/${item.id}`, { method: 'DELETE' })
    if (result.status === 409 || result.body?.error === 'referenced') {
      setMessage('참조 중이라 파일을 지우지 않았다.')
      return
    }
    if (result.status !== 200) {
      setMessage('파일을 지우지 못했다.')
      return
    }
    if (selected?.id === item.id) setSelected(null)
    setMessage('파일을 지웠다.')
    await load(page)
  }

  function choose(item: MediaItem) {
    setSelected(item)
    setAlt(item.alt)
    setCaption(item.caption)
  }

  async function onText(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!selected) return
    setPending(true)
    const result = await adminJson(`/api/admin/media/${selected.id}`, {
      method: 'PUT',
      body: JSON.stringify({ alt, caption }),
    })
    setPending(false)
    if (result.status !== 200) {
      setMessage('대체 텍스트를 저장하지 못했다.')
      return
    }
    setMessage('대체 텍스트와 캡션을 저장했다.')
    await load(page)
  }

  async function onUsage(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!selected || !entryId) return
    setPending(true)
    const result = await adminJson(`/api/admin/media/${selected.id}/usages`, {
      method: 'POST',
      body: JSON.stringify({ entryId }),
    })
    setPending(false)
    if (result.status === 422) {
      setMessage('초안이 있는 글만 사용처로 연결할 수 있다.')
      return
    }
    if (result.status !== 200) {
      setMessage('사용처를 연결하지 못했다.')
      return
    }
    setMessage('사용처를 연결했다. 이 파일은 참조 중이라 지우지 않는다.')
    await load(page)
  }

  if (!csrf) {
    return (
      <AdminFrame title="미디어 관리" label="이미지와 첨부 파일 · 공개 상태와 사용처" current="media">
        <p className="dg-item px-4 py-3" role="alert">로그인이 필요하다.</p>
        <p>
          <a className="dg-btn" href="/admin/login">소유자 로그인</a>
        </p>
      </AdminFrame>
    )
  }

  return (
    <AdminFrame title="미디어 관리" label="이미지와 첨부 파일 · 공개 상태와 사용처" current="media">
      <section
        className="dg-panel mb-4 flex flex-wrap items-center justify-between gap-4 border-dashed p-5"
        onDragOver={(event) => event.preventDefault()}
        onDrop={onDrop}
      >
        <div>
          <p className="mb-1 font-semibold">파일을 끌어놓거나 선택하세요</p>
          <p className="dg-label mb-0">허용 형식: PNG · JPG · WEBP · SVG · PDF · 최대 8MB</p>
        </div>
        <label className="dg-btn" htmlFor="media-file">
          업로드
          <input
            id="media-file"
            className="sr-only"
            type="file"
            accept=".png,.jpg,.jpeg,.webp,.svg,.pdf,image/png,image/jpeg,image/webp,image/svg+xml,application/pdf"
            disabled={pending}
            onChange={(event) => {
              const file = event.target.files?.item(0)
              if (file) void sendFile(file)
              event.target.value = ''
            }}
          />
        </label>
      </section>
      {message ? <p className="dg-item mb-4 px-4 py-3" role="status">{message}</p> : null}
      {items === null && !message ? <p>목록을 불러오는 중이다.</p> : null}
      {items && items.length === 0 ? <p className="dg-item px-4 py-3">파일이 없다.</p> : null}
      {items && items.length > 0 ? (
        <ul className="m-0 grid list-none gap-4 p-0 min-[40rem]:grid-cols-2 min-[64rem]:grid-cols-4">
          {items.map((item) => (
            <li key={item.id} className="dg-item grid gap-3 p-3">
              <button className="grid gap-3 text-left" type="button" onClick={() => choose(item)}>
                {selected?.id === item.id && preview ? (
                  <img className="h-28 w-full rounded-[var(--dg-radius-control)] object-cover" src={preview} alt={item.alt || item.originalName} />
                ) : (
                  <span className="dg-icon-tile h-28 w-full" aria-hidden="true">{item.mediaType === 'application/pdf' ? 'PDF' : 'IMG'}</span>
                )}
                <span className="truncate">{item.originalName}</span>
              </button>
              <div className="flex items-center justify-between gap-2 text-sm">
                <button className="dg-chip" type="button" onClick={() => void onVisibility(item)}>
                  {item.public ? '공개' : '비공개'}
                </button>
                <span>사용처 {item.usageCount}</span>
              </div>
              <button className="dg-btn-ghost" type="button" onClick={() => void onDelete(item)}>삭제</button>
            </li>
          ))}
        </ul>
      ) : null}
      {total > page * 8 ? (
        <button className="dg-btn mt-4" type="button" onClick={() => void load(page + 1)}>다음</button>
      ) : null}
      {selected ? (
        <div className="mt-4 grid gap-4 min-[48rem]:grid-cols-2">
          <form className="dg-panel grid gap-3 p-5" onSubmit={onText}>
            <h2 className="mb-0 text-xl">대체 텍스트 · 캡션</h2>
            <label htmlFor="media-alt">대체 텍스트</label>
            <input className="dg-input" id="media-alt" value={alt} onChange={(event) => setAlt(event.target.value)} />
            <label htmlFor="media-caption">캡션</label>
            <textarea className="dg-input" id="media-caption" value={caption} onChange={(event) => setCaption(event.target.value)} />
            <button className="dg-btn" type="submit" disabled={pending}>저장</button>
          </form>
          <form className="dg-panel grid gap-3 p-5" onSubmit={onUsage}>
            <h2 className="mb-0 text-xl">사용처</h2>
            <label htmlFor="media-entry">연결할 글</label>
            <select className="dg-input" id="media-entry" value={entryId} onChange={(event) => setEntryId(event.target.value)}>
              <option value="">글을 선택하세요</option>
              {entries.map((entry) => (
                <option key={entry.id} value={entry.id}>{entry.title}</option>
              ))}
            </select>
            <button className="dg-btn" type="submit" disabled={pending || !entryId}>사용처로 연결</button>
          </form>
        </div>
      ) : null}
    </AdminFrame>
  )
}
