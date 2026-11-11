/**
 * 개념: 수정 이력과 복원 화면.
 * 목적: 고른 수정본을 새 초안으로만 복사하고, 공개본은 그대로 둔다.
 * 사용처: /admin/entries/{id}/revisions.
 */
import { useEffect, useState } from 'react'
import { AdminFrame } from '../AdminFrame'
import { adminJson } from '../api'
import { blockLines, type Block } from '../editor/blocks'
import { readCsrf } from '../session'

/** 개념: 이력 목록의 한 줄. 본문 블록은 포함하지 않는다. */
type RevisionRow = {
  id: string
  version: number
  createdAt: string
  author: string
  draft: boolean
  published: boolean
}

/** 개념: 현재 초안. 버전은 복원 요청의 expectedVersion이 되고, 블록은 비교의 오른쪽이다. */
type EntryBody = {
  title: string
  version: number
  document?: { blocks?: Block[] }
}

/** 개념: 비교 창의 한 줄. same은 양쪽이 같고, remove는 고른 수정본만, add는 현재 초안만 가진 줄이다. */
type DiffLine = { kind: 'same' | 'remove' | 'add'; text: string }

/**
 * 개념: 수정본 시각의 화면 형식.
 * 목적: API의 UTC 시각을 `YYYY.MM.DD HH:mm`으로 보여 준다. 형식이 다르면 원문을 그대로 둔다.
 * 사용처: 이력 목록.
 */
function stamp(value: string): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})/.exec(value)
  return match ? `${match[1]}.${match[2]}.${match[3]} ${match[4]}:${match[5]}` : value
}

/**
 * 개념: 고른 수정본과 현재 초안의 줄 비교.
 * 목적: 같은 줄은 한 번만 맞추고, 고른 쪽에만 있으면 빠짐, 현재 초안에만 있으면 추가로 표시한다.
 * 사용처: 오른쪽 비교 창. 40줄이 넘으면 나머지는 미리보기에서 본다.
 */
function lineDiff(before: string[], after: string[]): DiffLine[] {
  // 현재 초안의 같은 줄을 두 번 맞추지 않기 위한 표시.
  const used = new Set<number>()
  const lines: DiffLine[] = []
  for (const line of before) {
    const index = after.findIndex((item, itemIndex) => item === line && !used.has(itemIndex))
    if (index >= 0) {
      used.add(index)
      lines.push({ kind: 'same', text: line })
    } else {
      lines.push({ kind: 'remove', text: line })
    }
  }
  after.forEach((line, index) => {
    if (!used.has(index)) lines.push({ kind: 'add', text: line })
  })
  return lines.slice(0, 40)
}

/**
 * 개념: 이력 목록과 복원 버튼.
 * 목적: 목록에는 본문을 넣지 않고, 고른 수정본만 따로 읽는다.
 * 사용처: App.
 */
export default function Revisions({ entryId }: { entryId: string }) {
  const csrf = readCsrf()
  const [rows, setRows] = useState<RevisionRow[] | null>(null)
  const [entry, setEntry] = useState<EntryBody | null>(null)
  const [selectedId, setSelectedId] = useState('')
  const [selectedBlocks, setSelectedBlocks] = useState<Block[] | null>(null)
  const [message, setMessage] = useState('')
  const [pending, setPending] = useState(false)

  // 목록과 현재 초안을 함께 읽는다. 목록 응답에는 블록이 없다.
  async function load() {
    const [list, current] = await Promise.all([
      adminJson<{ revisions?: RevisionRow[] }>(`/api/admin/entries/${entryId}/revisions`),
      adminJson<EntryBody>(`/api/admin/entries/${entryId}`),
    ])
    if (list.status === 401 || current.status === 401) return
    if (list.status === 404 || current.status === 404) {
      setMessage('글이 없다.')
      return
    }
    if (list.status !== 200 || current.status !== 200 || !list.body?.revisions || !current.body) {
      setMessage('이력을 불러오지 못했다.')
      return
    }
    setRows(list.body.revisions)
    setEntry(current.body)
    setSelectedId((currentId) => currentId || list.body?.revisions?.[0]?.id || '')
  }

  useEffect(() => {
    if (!csrf) return
    void load()
  }, [csrf, entryId])

  // 고른 수정본의 본문만 따로 요청한다.
  useEffect(() => {
    if (!csrf || !selectedId) return
    let cancelled = false
    async function loadOne() {
      const result = await adminJson<{ document?: { blocks?: Block[] } }>(
        `/api/admin/entries/${entryId}/revisions/${selectedId}`,
      )
      if (cancelled || result.status === 401) return
      if (result.status !== 200 || !result.body?.document) {
        setMessage('수정본을 불러오지 못했다.')
        return
      }
      setSelectedBlocks(result.body.document.blocks ?? [])
    }
    void loadOne()
    return () => {
      cancelled = true
    }
  }, [csrf, entryId, selectedId])

  // 복원은 현재 초안 버전을 기대값으로 보낸다. 409면 서버 목록을 다시 읽어 화면을 맞춘다.
  async function onRestore() {
    if (!entry || !selectedId) return
    setPending(true)
    setMessage('')
    const result = await adminJson(`/api/admin/entries/${entryId}/restore`, {
      method: 'POST',
      body: JSON.stringify({ revisionId: selectedId, expectedVersion: entry.version }),
    })
    setPending(false)
    if (result.status === 409) {
      setMessage('다른 수정이 먼저 저장되었다.')
      await load()
      return
    }
    if (result.status !== 200) {
      setMessage('복원하지 못했다.')
      return
    }
    setMessage('새 초안을 만들었다. 공개본은 바뀌지 않았다.')
    setSelectedId('')
    await load()
  }

  const selected = rows?.find((row) => row.id === selectedId)
  // 왼쪽이 고른 수정본, 오른쪽이 현재 초안이다.
  const diff = lineDiff(blockLines(selectedBlocks ?? []), blockLines(entry?.document?.blocks ?? []))

  // 세션이 없으면 이력 API를 호출하지 않고 로그인 안내만 보여 준다.
  if (!csrf) {
    return (
      <AdminFrame title="수정 이력과 복원" current="entries">
        <p className="dg-item px-4 py-3" role="alert">로그인이 필요하다.</p>
        <p>
          <a className="dg-btn" href="/admin/login">소유자 로그인</a>
        </p>
      </AdminFrame>
    )
  }

  return (
    <AdminFrame
      title="수정 이력과 복원"
      label={entry ? `게시물 / ${entry.title}` : '게시물'}
      current="entries"
      action={
        <button className="dg-btn-ghost" type="button" onClick={() => void load()}>
          새로고침
        </button>
      }
    >
      {message ? <p className="dg-item mb-4 px-4 py-3" role="status">{message}</p> : null}
      {rows === null && !message ? <p>이력을 불러오는 중이다.</p> : null}
      {rows && rows.length === 0 ? <p className="dg-item px-4 py-3">수정 이력이 없다.</p> : null}
      {rows && rows.length > 0 ? (
        <div className="grid gap-4 min-[64rem]:grid-cols-[18rem_1fr]">
          <section className="dg-panel p-4">
            <h2 className="mb-3 text-lg">수정본 · 작성 시각</h2>
            <ul className="m-0 grid list-none gap-2 p-0">
              {rows.map((row) => (
                <li key={row.id}>
                  <button className="dg-item flex w-full items-center justify-between gap-2 px-3 py-2 text-left" type="button" onClick={() => setSelectedId(row.id)}>
                    <span>
                      r-[{String(row.version).padStart(2, '0')}]
                      <br />
                      <span className="text-sm text-quiet">{stamp(row.createdAt)} · {row.author}</span>
                    </span>
                    {/* 현재 초안이 공개본이기도 하면 초안 표시를 우선한다. */}
                    <span className="dg-chip">{row.draft ? '현재 초안' : row.published ? '공개본' : '초안'}</span>
                  </button>
                </li>
              ))}
            </ul>
          </section>
          <section className="dg-panel grid gap-3 p-4">
            <h2 className="mb-0 text-lg">
              선택 수정본 ↔ 현재 초안
              {selected ? ` · r-[${String(selected.version).padStart(2, '0')}]` : ''}
            </h2>
            <ul className="m-0 grid list-none gap-2 p-0">
              {diff.map((line, index) => (
                <li key={`${line.kind}-${index}`} className="dg-item px-3 py-2">
                  {line.kind === 'remove' ? `- ${line.text}` : line.kind === 'add' ? `+ ${line.text}` : line.text}
                </li>
              ))}
            </ul>
            <button className="dg-btn" type="button" onClick={() => void onRestore()} disabled={pending || !selectedId}>
              이 버전을 새 초안으로 복원
            </button>
            <p className="mb-0 text-sm text-quiet">복원만으로 공개본은 바뀌지 않습니다.</p>
          </section>
        </div>
      ) : null}
    </AdminFrame>
  )
}
