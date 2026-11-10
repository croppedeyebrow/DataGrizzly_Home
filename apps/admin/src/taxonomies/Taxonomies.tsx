/**
 * 개념: 분류와 시리즈 관리 화면.
 * 목적: 주제, 태그, 시리즈를 고치고, 분류를 지워도 글은 남긴다.
 * 사용처: /admin/taxonomies.
 */
import { useEffect, useState, type FormEvent } from 'react'
import { AdminFrame } from '../AdminFrame'
import { adminJson } from '../api'
import { readCsrf } from '../session'

type Kind = 'topic' | 'tag' | 'series'

type TaxonomyRow = {
  id: string
  kind: Kind
  slug: string
  title: string
  description: string
  entryCount: number
}

type LinkedEntry = { id: string; title: string; position?: number }

type TaxonomyDetail = TaxonomyRow & { entries: LinkedEntry[] }

const tabs: { id: Kind; label: string }[] = [
  { id: 'topic', label: '주제' },
  { id: 'tag', label: '태그' },
  { id: 'series', label: '시리즈' },
]

/**
 * 개념: 분류 표와 시리즈 순서.
 * 목적: 순서 저장은 현재 목록 전체를 한 번에 보낸다.
 * 사용처: App의 taxonomies 경로.
 */
export default function Taxonomies() {
  const csrf = readCsrf()
  const [kind, setKind] = useState<Kind>('topic')
  const [rows, setRows] = useState<TaxonomyRow[] | null>(null)
  const [selected, setSelected] = useState<TaxonomyDetail | null>(null)
  const [order, setOrder] = useState<LinkedEntry[]>([])
  const [message, setMessage] = useState('')
  const [open, setOpen] = useState(false)
  const [title, setTitle] = useState('')
  const [slug, setSlug] = useState('')
  const [pending, setPending] = useState(false)

  async function loadList(nextKind: Kind) {
    const result = await adminJson<{ taxonomies?: TaxonomyRow[] }>(`/api/admin/taxonomies?kind=${nextKind}`)
    if (result.status === 401) return
    if (result.status !== 200 || !result.body?.taxonomies) {
      setMessage(result.status === 0 ? '서버에 연결하지 못했다.' : '목록을 불러오지 못했다.')
      setRows(null)
      return
    }
    setRows(result.body.taxonomies)
  }

  useEffect(() => {
    if (!csrf) return
    void loadList(kind)
  }, [csrf, kind])

  async function onCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setPending(true)
    setMessage('')
    const result = await adminJson('/api/admin/taxonomies', {
      method: 'POST',
      body: JSON.stringify({ kind, slug, title, description: '' }),
    })
    setPending(false)
    if (result.status === 422) {
      setMessage('이름과 슬러그를 다시 확인해 달라.')
      return
    }
    if (result.status === 409) {
      setMessage('같은 슬러그가 이미 있다.')
      return
    }
    if (result.status !== 201) {
      setMessage('항목을 만들지 못했다.')
      return
    }
    setTitle('')
    setSlug('')
    setOpen(false)
    setMessage('항목을 만들었다.')
    await loadList(kind)
  }

  async function openRow(id: string) {
    setMessage('')
    const result = await adminJson<TaxonomyDetail>(`/api/admin/taxonomies/${id}`)
    if (result.status !== 200 || !result.body) {
      setMessage('항목을 불러오지 못했다.')
      return
    }
    setSelected(result.body)
    setTitle(result.body.title)
    setSlug(result.body.slug)
    setOrder(result.body.entries)
  }

  async function onSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!selected) return
    setPending(true)
    setMessage('')
    const result = await adminJson(`/api/admin/taxonomies/${selected.id}`, {
      method: 'PUT',
      body: JSON.stringify({ slug, title, description: selected.description }),
    })
    setPending(false)
    if (result.status === 409) {
      setMessage('같은 슬러그가 이미 있다.')
      return
    }
    if (result.status !== 200) {
      setMessage('항목을 저장하지 못했다.')
      return
    }
    setMessage('항목을 저장했다.')
    await loadList(kind)
    await openRow(selected.id)
  }

  async function onDelete() {
    if (!selected) return
    if (!window.confirm('분류를 지우면 연결만 사라지고 글은 남습니다.')) return
    setPending(true)
    const result = await adminJson(`/api/admin/taxonomies/${selected.id}`, { method: 'DELETE' })
    setPending(false)
    if (result.status !== 200) {
      setMessage('항목을 지우지 못했다.')
      return
    }
    setSelected(null)
    setOrder([])
    setMessage('분류를 지웠다. 연결된 글은 그대로다.')
    await loadList(kind)
  }

  function move(index: number, delta: number) {
    const next = index + delta
    if (next < 0 || next >= order.length) return
    const copy = order.slice()
    const [item] = copy.splice(index, 1)
    copy.splice(next, 0, item)
    setOrder(copy)
  }

  async function onOrder() {
    if (!selected) return
    setPending(true)
    setMessage('')
    const result = await adminJson(`/api/admin/series/${selected.id}/order`, {
      method: 'PUT',
      body: JSON.stringify({ entryIds: order.map((item) => item.id) }),
    })
    setPending(false)
    if (result.status !== 200) {
      setMessage('순서를 저장하지 못했다.')
      await openRow(selected.id)
      return
    }
    setMessage('시리즈 순서를 저장했다.')
    await openRow(selected.id)
  }

  if (!csrf) {
    return (
      <AdminFrame title="분류와 시리즈 관리" label="삭제해도 원문 글은 보존됩니다" current="taxonomies">
        <p className="dg-item px-4 py-3" role="alert">로그인이 필요하다.</p>
        <p>
          <a className="dg-btn" href="/admin/login">소유자 로그인</a>
        </p>
      </AdminFrame>
    )
  }

  return (
    <AdminFrame
      title="분류와 시리즈 관리"
      label="삭제해도 원문 글은 보존됩니다"
      current="taxonomies"
      action={
        <button className="dg-btn" type="button" onClick={() => setOpen((value) => !value)}>
          + 새 항목
        </button>
      }
    >
      <div className="mb-4 flex flex-wrap gap-2" role="tablist">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            className={tab.id === kind ? 'dg-btn' : 'dg-btn-ghost'}
            type="button"
            role="tab"
            aria-selected={tab.id === kind}
            onClick={() => {
              setKind(tab.id)
              setSelected(null)
              setOrder([])
            }}
          >
            {tab.label}
          </button>
        ))}
      </div>
      {open ? (
        <form className="dg-panel mb-4 grid gap-3 p-5" onSubmit={onCreate}>
          <label htmlFor="tax-title">이름</label>
          <input className="dg-input" id="tax-title" value={title} onChange={(event) => setTitle(event.target.value)} />
          <label htmlFor="tax-slug">slug</label>
          <input className="dg-input" id="tax-slug" value={slug} onChange={(event) => setSlug(event.target.value)} />
          <button className="dg-btn" type="submit" disabled={pending}>항목 만들기</button>
        </form>
      ) : null}
      {message ? <p className="dg-item mb-4 px-4 py-3" role="status">{message}</p> : null}
      <div className="grid gap-4 min-[64rem]:grid-cols-[1fr_18rem]">
        <div className="dg-panel overflow-x-auto p-4">
          {rows === null && !message ? <p>목록을 불러오는 중이다.</p> : null}
          {rows && rows.length === 0 ? <p className="mb-0">분류가 없다.</p> : null}
          {rows && rows.length > 0 ? (
            <table className="w-full border-collapse text-left">
              <thead>
                <tr className="text-sm text-quiet">
                  <th className="px-2 py-2 font-medium">이름</th>
                  <th className="px-2 py-2 font-medium">slug</th>
                  <th className="px-2 py-2 font-medium">연결 글</th>
                  <th className="px-2 py-2 font-medium">작업</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr key={row.id} className="border-t border-white/10">
                    <td className="px-2 py-3">{row.title}</td>
                    <td className="px-2 py-3">{row.slug}</td>
                    <td className="px-2 py-3">{row.entryCount}편</td>
                    <td className="px-2 py-3">
                      <button className="dg-btn-ghost" type="button" onClick={() => void openRow(row.id)}>편집</button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : null}
        </div>
        {selected ? (
          <aside className="dg-panel grid h-fit gap-3 p-4">
            <form className="grid gap-3" onSubmit={onSave}>
              <h2 className="mb-0 text-lg">{kind === 'series' ? `시리즈 읽는 순서 · ${selected.title}` : '편집'}</h2>
              <label htmlFor="edit-tax-title">이름</label>
              <input className="dg-input" id="edit-tax-title" value={title} onChange={(event) => setTitle(event.target.value)} />
              <label htmlFor="edit-tax-slug">slug</label>
              <input className="dg-input" id="edit-tax-slug" value={slug} onChange={(event) => setSlug(event.target.value)} />
              <button className="dg-btn" type="submit" disabled={pending}>저장</button>
            </form>
            {kind === 'series' ? (
              <>
                {order.length === 0 ? <p className="mb-0 text-sm">연결된 글이 없다.</p> : null}
                <ol className="m-0 grid list-none gap-2 p-0">
                  {order.map((item, index) => (
                    <li key={item.id} className="dg-item flex items-center justify-between gap-2 px-3 py-2">
                      <span>{String(index + 1).padStart(2, '0')} · {item.title}</span>
                      <span className="flex gap-1">
                        <button className="dg-btn-ghost" type="button" onClick={() => move(index, -1)}>위로</button>
                        <button className="dg-btn-ghost" type="button" onClick={() => move(index, 1)}>아래로</button>
                      </span>
                    </li>
                  ))}
                </ol>
                {order.length > 1 ? (
                  <button className="dg-btn" type="button" onClick={() => void onOrder()} disabled={pending}>순서 저장</button>
                ) : null}
              </>
            ) : null}
            <button className="dg-btn-ghost" type="button" onClick={() => void onDelete()} disabled={pending}>삭제</button>
          </aside>
        ) : null}
      </div>
    </AdminFrame>
  )
}
