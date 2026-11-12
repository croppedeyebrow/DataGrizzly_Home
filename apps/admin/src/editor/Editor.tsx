/**
 * 개념: 블록 편집 화면.
 * 목적: 계약된 블록을 자동 저장하고, 충돌과 실패 때 로컬 내용을 유지한다.
 * 사용처: /admin/entries/{id}/edit.
 */
import { useEffect, useState } from 'react'
import { AdminFrame } from '../AdminFrame'
import { clearOwnerSession, draftKey, readCsrf } from '../session'
import { canAutosave, reduceSave, type SaveState } from './autosave'
import { blankBlock, blockLines, type Block, type EntryDocument } from './blocks'

type LoadedEntry = {
  id: string
  title: string
  summary: string
  kind: string
  version: number
  projectId: string | null
  draftRevisionId: string | null
  publishedRevisionId: string | null
  document: EntryDocument | null
}

type LocalDraft = {
  title: string
  summary: string
  blocks: Block[]
  baseVersion: number
}

const INITIAL_SAVE: SaveState = {
  status: 'saved',
  generation: 0,
  sentGeneration: 0,
  expectedVersion: 1,
  conflictVersion: null,
}

/**
 * 개념: 한 글의 편집기.
 * 목적: 서버 초안과 이 탭의 미저장 초안을 구분해서 보여 준다.
 * 사용처: App의 edit 경로.
 */
export default function Editor({ entryId }: { entryId: string }) {
  const csrf = readCsrf()
  const [entry, setEntry] = useState<LoadedEntry | null>(null)
  const [title, setTitle] = useState('')
  const [summary, setSummary] = useState('')
  const [blocks, setBlocks] = useState<Block[]>([])
  const [saveState, setSaveState] = useState<SaveState>(INITIAL_SAVE)
  const [serverCopy, setServerCopy] = useState<LocalDraft | null>(null)
  const [message, setMessage] = useState('')
  const [publishNote, setPublishNote] = useState('')
  const [loadError, setLoadError] = useState('')

  useEffect(() => {
    if (!csrf) return
    let cancelled = false
    async function load() {
      try {
        const response = await fetch(`/api/admin/entries/${entryId}`, {
          credentials: 'include',
          headers: { 'x-csrf-token': csrf },
        })
        if (response.status === 401) {
          clearOwnerSession()
          window.location.assign('/admin/login')
          return
        }
        if (response.status === 404) {
          if (!cancelled) setLoadError('글을 찾지 못했다.')
          return
        }
        if (!response.ok) {
          if (!cancelled) setLoadError('글을 불러오지 못했다.')
          return
        }
        const body = (await response.json()) as LoadedEntry
        if (cancelled) return
        const server = draftFromEntry(body)
        const local = readLocalDraft(entryId)
        setEntry(body)
        setServerCopy(server)
        if (local) {
          setTitle(local.title)
          setSummary(local.summary)
          setBlocks(local.blocks)
          setSaveState({
            status: local.baseVersion === body.version ? 'failed' : 'conflict',
            generation: 1,
            sentGeneration: 0,
            expectedVersion: body.version,
            conflictVersion: local.baseVersion === body.version ? null : body.version,
          })
          setMessage('이 탭에 저장되지 않은 수정이 남아 있다.')
          return
        }
        setTitle(server.title)
        setSummary(server.summary)
        setBlocks(server.blocks)
        setSaveState(reduceSave(INITIAL_SAVE, { type: 'load', version: body.version }))
      } catch {
        if (!cancelled) setLoadError('서버에 연결하지 못했다.')
      }
    }
    void load()
    return () => {
      cancelled = true
    }
  }, [csrf, entryId])

  useEffect(() => {
    if (!canAutosave(saveState) || !entry) return
    const timer = window.setTimeout(() => {
      void sendDraft('autosave')
    }, 800)
    return () => window.clearTimeout(timer)
  }, [saveState, title, summary, blocks, entry])

  useEffect(() => {
    function onLeave(event: BeforeUnloadEvent) {
      if (saveState.status === 'saved' || saveState.generation === 0) return
      event.preventDefault()
    }
    window.addEventListener('beforeunload', onLeave)
    return () => window.removeEventListener('beforeunload', onLeave)
  }, [saveState])

  async function sendDraft(reason: 'autosave' | 'explicit') {
    if (!entry) return
    if (reason === 'autosave' && !canAutosave(saveState)) return
    if (saveState.status === 'conflict' || saveState.status === 'saving') return
    const generation = saveState.generation
    const snapshot = { title, summary, blocks, baseVersion: saveState.expectedVersion }
    sessionStorage.setItem(draftKey(entry.id), JSON.stringify(snapshot))
    setSaveState((current) => reduceSave(current, { type: 'send' }))
    try {
      const response = await fetch(`/api/admin/entries/${entry.id}`, {
        method: 'PUT',
        credentials: 'include',
        headers: { 'content-type': 'application/json', 'x-csrf-token': csrf },
        body: JSON.stringify({
          expectedVersion: saveState.expectedVersion,
          title,
          summary,
          blocks,
          metadata: metadataFor(entry.kind),
        }),
      })
      if (response.status === 409) {
        const body = (await response.json()) as { version?: number; title?: string; summary?: string; document?: EntryDocument }
        setServerCopy({
          title: body.title ?? '',
          summary: body.summary ?? '',
          blocks: body.document?.blocks ?? [],
          baseVersion: body.version ?? saveState.expectedVersion,
        })
        setSaveState((current) => reduceSave(current, { type: 'conflict', version: body.version ?? current.expectedVersion }))
        setMessage('다른 탭의 저장본이 더 최신이다. 이 탭의 내용은 유지했다.')
        return
      }
      if (response.status === 422) {
        setSaveState((current) => reduceSave(current, { type: 'invalid' }))
        setMessage('저장할 수 없는 블록이 있다. 입력은 그대로 두었다.')
        return
      }
      if (!response.ok) {
        setSaveState((current) => reduceSave(current, { type: 'failed' }))
        setMessage('저장하지 못했다. 입력은 그대로 두었다.')
        return
      }
      const body = (await response.json()) as { version?: number; draftRevisionId?: string }
      const version = body.version ?? saveState.expectedVersion
      if (body.draftRevisionId) {
        setEntry((current) => (current ? { ...current, draftRevisionId: body.draftRevisionId ?? null, version } : current))
      }
      setSaveState((current) => reduceSave(current, { type: 'saved', generation, version }))
      setSaveState((current) => {
        if (current.status === 'saved' && current.generation === generation) {
          sessionStorage.removeItem(draftKey(entry.id))
          setMessage('')
        }
        return current
      })
      setServerCopy({ title, summary, blocks, baseVersion: version })
    } catch {
      setSaveState((current) => reduceSave(current, { type: 'failed' }))
      setMessage('서버에 연결하지 못했다. 입력은 이 탭에 남아 있다.')
    }
  }

  function changeBlocks(next: Block[]) {
    setBlocks(next)
    setSaveState((current) => reduceSave(current, { type: 'edit' }))
    if (entry) {
      sessionStorage.setItem(
        draftKey(entry.id),
        JSON.stringify({ title, summary, blocks: next, baseVersion: saveState.expectedVersion }),
      )
    }
  }

  function restoreServer() {
    if (!serverCopy || !entry) return
    setTitle(serverCopy.title)
    setSummary(serverCopy.summary)
    setBlocks(serverCopy.blocks)
    sessionStorage.removeItem(draftKey(entry.id))
    setSaveState(reduceSave(INITIAL_SAVE, { type: 'load', version: serverCopy.baseVersion }))
    setMessage('서버에 저장된 초안을 다시 열었다.')
  }

  function acceptConflict() {
    setSaveState((current) => reduceSave(current, { type: 'acceptServerVersion' }))
    setMessage('서버 버전을 기준으로 삼았다. 저장을 눌러야 이 탭의 내용이 올라간다.')
  }

  if (!csrf) {
    return (
      <AdminFrame title="블록 편집기">
        <p className="dg-item px-4 py-3" role="alert">로그인이 필요하다.</p>
        <p>
          <a className="dg-btn" href="/admin/login">소유자 로그인</a>
        </p>
      </AdminFrame>
    )
  }
  if (loadError) {
    return (
      <AdminFrame title="블록 편집기">
        <p className="dg-item px-4 py-3" role="alert">{loadError}</p>
        <p>
          <a href="/admin/entries">게시물 목록</a>
        </p>
      </AdminFrame>
    )
  }
  if (!entry) {
    return (
      <AdminFrame title="블록 편집기">
        <p>글을 불러오는 중이다.</p>
      </AdminFrame>
    )
  }

  // 저장이 끝난 초안만 접수한다. 같은 수정본의 반복 클릭은 같은 키를 다시 보낸다.
  async function onAccept() {
    if (!entry?.draftRevisionId || saveState.status !== 'saved') return
    const storageKey = `dg-publish:${entry.id}:${entry.draftRevisionId}`
    let key = sessionStorage.getItem(storageKey) ?? ''
    if (!/^[A-Za-z0-9._:-]{8,80}$/.test(key)) {
      key = `edit.${crypto.randomUUID()}`
      sessionStorage.setItem(storageKey, key)
    }
    setPublishNote('')
    try {
      const response = await fetch(`/api/admin/entries/${entry.id}/publications`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'content-type': 'application/json', 'x-csrf-token': csrf },
        body: JSON.stringify({ schemaVersion: 1, revisionId: entry.draftRevisionId, idempotencyKey: key }),
      })
      if (response.status === 202) {
        setPublishNote('접수했다. 공개본은 바뀌지 않았다.')
        return
      }
      if (response.status === 409) {
        setPublishNote('같은 접수 키에 다른 수정본이 있다.')
        return
      }
      setPublishNote('발행 작업으로 접수하지 못했다.')
    } catch {
      setPublishNote('서버에 연결하지 못했다.')
    }
  }

  // 공개 포인터를 바로 비운다. 202는 외부 캐시가 지워졌다는 뜻이 아니다.
  async function onWithdraw() {
    if (!csrf || !entry) return
    try {
      const response = await fetch(`/api/admin/entries/${entry.id}/unpublish`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'content-type': 'application/json', 'x-csrf-token': csrf },
        body: JSON.stringify({ schemaVersion: 1, expectedVersion: entry.version }),
      })
      if (response.status === 202) {
        setEntry((current) => (current ? { ...current, publishedRevisionId: null } : current))
        setPublishNote('철회를 접수했다. 공개 포인터는 비웠다. 외부 캐시 삭제는 보장하지 않는다.')
        return
      }
      if (response.status === 409) {
        setPublishNote('다른 수정이 먼저 저장되었다.')
        return
      }
      setPublishNote('철회하지 못했다.')
    } catch {
      setPublishNote('서버에 연결하지 못했다.')
    }
  }

  return (
    <AdminFrame title="블록 편집기">
      <p className="dg-chip mb-4">{statusText(saveState.status)}</p>
      {/* 글을 불러온 뒤에만 미리보기, 이력, 발행 접수로 이동한다. 접수는 공개 완료가 아니다. */}
      <p className="mb-4 flex flex-wrap gap-2">
        <a className="dg-btn-ghost" href={`/admin/entries/${entry.id}/preview`}>미리보기</a>
        <a className="dg-btn-ghost" href={`/admin/entries/${entry.id}/revisions`}>수정 이력</a>
        <button className="dg-btn" type="button" onClick={() => void onAccept()} disabled={saveState.status !== 'saved' || !entry.draftRevisionId}>
          이 초안을 발행 작업으로 접수
        </button>
        {entry.publishedRevisionId ? (
          <button className="dg-btn-ghost" type="button" onClick={() => void onWithdraw()}>
            이 글을 철회
          </button>
        ) : null}
      </p>
      {saveState.status !== 'saved' ? <p className="text-sm text-quiet">저장되지 않은 편집은 접수하지 않는다.</p> : null}
      {publishNote ? (
        <p className="dg-item mb-4 px-4 py-3" role="status">
          {publishNote}{' '}
          <a href="/admin/publications">발행 작업 보기</a>
        </p>
      ) : null}
      <p>
        종류 {entry.kind}. 주제는 아직 없다. 연결된 프로젝트는 {entry.projectId ?? '없다'}. 공개 수정본은{' '}
        {entry.publishedRevisionId ? '있다' : '없다'}.
      </p>
      <label htmlFor="draft-title">글 제목을 입력하세요</label>
      <input
        id="draft-title"
        placeholder="글 제목을 입력하세요"
        value={title}
        onChange={(event) => {
          setTitle(event.target.value)
          setSaveState((current) => reduceSave(current, { type: 'edit' }))
        }}
      />
      <label htmlFor="draft-summary">요약</label>
      <textarea
        id="draft-summary"
        value={summary}
        onChange={(event) => {
          setSummary(event.target.value)
          setSaveState((current) => reduceSave(current, { type: 'edit' }))
        }}
      />
      <section>
        {blocks.map((block, index) => (
          <BlockEditor
            key={String(block.id ?? index)}
            block={block}
            onChange={(next) => changeBlocks(blocks.map((item, itemIndex) => (itemIndex === index ? next : item)))}
            onRemove={() => changeBlocks(blocks.filter((_, itemIndex) => itemIndex !== index))}
          />
        ))}
      </section>
      <label htmlFor="block-type">+ 블록 추가</label>
      <select
        id="block-type"
        defaultValue="paragraph"
        onChange={(event) => {
          changeBlocks([...blocks, blankBlock(event.target.value)])
          event.target.value = 'paragraph'
        }}
      >
        <option value="paragraph">문단</option>
        <option value="heading">제목</option>
        <option value="list">목록</option>
        <option value="quote">인용</option>
        <option value="table">표</option>
        <option value="image">이미지</option>
        <option value="code">코드</option>
        <option value="math">수식</option>
        <option value="callout">콜아웃</option>
        <option value="related">관련 글</option>
      </select>
      <button className="dg-btn" type="button" onClick={() => void sendDraft('explicit')} disabled={saveState.status === 'saving' || saveState.status === 'conflict'}>
        초안 저장
      </button>
      {saveState.status === 'conflict' ? (
        <section>
          <h2>서버 최신본</h2>
          <p>{serverCopy?.title}</p>
          <ul>
            {blockLines(serverCopy?.blocks ?? []).map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
          <button className="dg-btn" type="button" onClick={acceptConflict}>
            서버 버전을 기준으로 내 내용을 저장할 준비
          </button>
          <button className="dg-btn-ghost" type="button" onClick={restoreServer}>
            서버 저장본으로 되돌리기
          </button>
        </section>
      ) : null}
      {message ? <p className="dg-item mt-4 px-4 py-3" role="alert">{message}</p> : null}
    </AdminFrame>
  )
}

/**
 * 개념: 블록 필드 하나.
 * 목적: 코드와 수식은 텍스트로만 고친다.
 * 사용처: Editor의 블록 목록.
 */
function BlockEditor({
  block,
  onChange,
  onRemove,
}: {
  block: Block
  onChange: (block: Block) => void
  onRemove: () => void
}) {
  const type = String(block.type)
  function text(key: string) {
    return typeof block[key] === 'string' ? String(block[key]) : ''
  }
  return (
    <fieldset className="dg-item">
      <legend>{type}</legend>
      {type === 'paragraph' || type === 'quote' || type === 'callout' || type === 'code' || type === 'math' ? (
        <textarea
          aria-label={type}
          value={text(type === 'math' ? 'tex' : 'text')}
          onChange={(event) => onChange({ ...block, [type === 'math' ? 'tex' : 'text']: event.target.value })}
        />
      ) : null}
      {type === 'heading' ? (
        <input aria-label="heading" value={text('text')} onChange={(event) => onChange({ ...block, text: event.target.value, anchor: anchorFrom(event.target.value) })} />
      ) : null}
      {type === 'list' ? (
        <textarea
          aria-label="list"
          value={Array.isArray(block.items) ? block.items.map(String).join('\n') : ''}
          onChange={(event) => onChange({ ...block, items: event.target.value.split('\n') })}
        />
      ) : null}
      {type === 'table' ? (
        <textarea
          aria-label="table"
          value={[
            Array.isArray(block.header) ? block.header.map(String).join(' | ') : '',
            ...(Array.isArray(block.rows) ? block.rows.map((row) => (Array.isArray(row) ? row.map(String).join(' | ') : '')) : []),
          ].join('\n')}
          onChange={(event) => onChange({ ...block, ...tableFrom(event.target.value) })}
        />
      ) : null}
      {type === 'image' ? (
        <>
          <input aria-label="alt" value={text('alt')} onChange={(event) => onChange({ ...block, alt: event.target.value })} />
          <input aria-label="url" value={text('url')} onChange={(event) => onChange({ ...block, url: event.target.value })} />
        </>
      ) : null}
      {type === 'code' ? (
        <input aria-label="language" value={text('language')} onChange={(event) => onChange({ ...block, language: event.target.value })} />
      ) : null}
      {type === 'related' ? (
        <input aria-label="entryId" value={text('entryId')} onChange={(event) => onChange({ ...block, entryId: event.target.value })} />
      ) : null}
      <button type="button" onClick={onRemove}>
        블록 삭제
      </button>
    </fieldset>
  )
}

/**
 * 개념: 서버 응답을 편집 값으로 바꾼다.
 * 목적: 저장된 초안을 새로고침 뒤 같은 필드로 연다.
 * 사용처: 편집 화면의 첫 로드.
 */
function draftFromEntry(entry: LoadedEntry): LocalDraft {
  return {
    title: entry.document?.title || entry.title,
    summary: entry.summary,
    blocks: entry.document?.blocks ?? [],
    baseVersion: entry.version,
  }
}

/**
 * 개념: 이 탭에 남은 미저장 초안을 읽는다.
 * 목적: 서버 저장과 브라우저 임시 보존을 같은 값으로 보지 않는다.
 * 사용처: 편집 화면 로드.
 */
function readLocalDraft(entryId: string): LocalDraft | null {
  const raw = sessionStorage.getItem(draftKey(entryId))
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as LocalDraft
    if (!Array.isArray(parsed.blocks)) return null
    return parsed
  } catch {
    return null
  }
}

/**
 * 개념: 글 종류에 맞는 메타데이터.
 * 목적: 로그가 아닌 글에 로그 필드를 섞지 않는다.
 * 사용처: 저장 요청.
 */
function metadataFor(kind: string) {
  if (kind === 'log') {
    return {
      type: 'log',
      projectId: '00000000-0000-0000-0000-000000000000',
      stage: 'planning',
      displayOrder: 0,
    }
  }
  if (kind === 'paper' || kind === 'news') return { type: kind }
  return { type: 'note' }
}

/**
 * 개념: 제목 블록의 앵커.
 * 목적: 화면에서 앵커 필드를 따로 받지 않아도 계약 형식을 맞춘다.
 * 사용처: heading 입력.
 */
function anchorFrom(value: string): string {
  const anchor = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
  return anchor || 'section'
}

/**
 * 개념: 표 텍스트를 header와 rows로 나눈다.
 * 목적: 표 편집기가 별도 표 모델을 만들지 않게 한다.
 * 사용처: table 입력.
 */
function tableFrom(value: string): { header: string[]; rows: string[][] } {
  const lines = value.split('\n').filter((line) => line.length > 0)
  const cells = (line: string) => line.split('|').map((cell) => cell.trim())
  return {
    header: cells(lines[0] ?? '열'),
    rows: lines.slice(1).map(cells),
  }
}

/**
 * 개념: 저장 상태 문장.
 * 목적: 저장 중, 완료, 실패를 같은 문장으로 합치지 않는다.
 * 사용처: 편집 화면 상단.
 */
function statusText(status: SaveState['status']): string {
  if (status === 'saving') return '저장 중이다.'
  if (status === 'saved') return '저장 완료 · 공개본은 변경되지 않았습니다'
  if (status === 'failed') return '저장에 실패했다.'
  if (status === 'conflict') return '저장이 충돌했다.'
  if (status === 'invalid') return '형식이 맞지 않아 저장하지 않았다.'
  return '저장 대기 중이다.'
}
