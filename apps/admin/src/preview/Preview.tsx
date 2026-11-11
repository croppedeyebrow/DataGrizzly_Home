/**
 * 개념: 초안 미리보기 화면.
 * 목적: 로그인한 소유자에게만 초안을 공개 렌더러로 보여주고, 공개본과 분리한다.
 * 사용처: /admin/entries/{id}/preview.
 */
import { useEffect, useState } from 'react'
import { BlockList } from '../../../web/src/site/blocks.tsx'
import { AdminFrame } from '../AdminFrame'
import { adminJson } from '../api'
import { readCsrf } from '../session'
import { contractToView } from './contractView'

/** 개념: 미리보기 API가 돌려주는 초안. 본문은 document.blocks에만 있다. */
type PreviewBody = {
  revisionId: string
  slug: string
  kind: string
  title: string
  summary: string
  document?: { blocks?: unknown }
}

/**
 * 개념: 글 종류의 화면 이름.
 * 목적: 저장값 note, paper, news, log를 공개 화면과 같은 한글 라벨로 보여 준다.
 * 사용처: 미리보기 본문 위.
 */
function kindLabel(kind: string): string {
  if (kind === 'paper') return '논문 리뷰'
  if (kind === 'news') return '뉴스 정리'
  if (kind === 'log') return '과정 기록'
  return '노트'
}

/**
 * 개념: 이 초안이 나중에 놓일 공개 경로.
 * 목적: 주소 칸에 초안임을 같이 보여 주고, 공개 파일을 바꾸지는 않는다.
 * 사용처: 미리보기 상단.
 */
function publicPath(kind: string, slug: string): string {
  if (kind === 'log') return `/logs/${slug}`
  if (kind === 'paper' || kind === 'news') return `/research/${slug}`
  return `/notes/${slug}`
}

/**
 * 개념: 미리보기 한 화면.
 * 목적: 데스크톱과 모바일 폭만 바꾸고, 검색 로봇 메타는 접근 통제가 아니다.
 * 사용처: App.
 */
export default function Preview({ entryId }: { entryId: string }) {
  const csrf = readCsrf()
  const [preview, setPreview] = useState<PreviewBody | null>(null)
  const [message, setMessage] = useState('')
  // 폭 전환은 표시만 바꾸는 것이 아니라 미리보기 칸의 최대 너비를 390px로 줄인다.
  const [width, setWidth] = useState<'desktop' | 'mobile'>('desktop')

  // 이 화면에 머무는 동안만 검색 로봇 메타를 붙인다. 세션 검사를 대신하지 않는다.
  useEffect(() => {
    const meta = document.createElement('meta')
    meta.name = 'robots'
    meta.content = 'noindex'
    document.head.append(meta)
    return () => meta.remove()
  }, [])

  // 로그인한 뒤에만 초안을 요청한다. 401이면 공통 API 처리가 로그인으로 보낸다.
  useEffect(() => {
    if (!csrf) return
    let cancelled = false
    async function load() {
      const result = await adminJson<PreviewBody>(`/api/admin/entries/${entryId}/preview`)
      if (cancelled || result.status === 401) return
      if (result.status === 404) {
        setMessage('초안이 없다.')
        return
      }
      if (result.status !== 200 || !result.body) {
        setMessage(result.status === 0 ? '서버에 연결하지 못했다.' : '미리보기를 불러오지 못했다.')
        return
      }
      setPreview(result.body)
    }
    void load()
    return () => {
      cancelled = true
    }
  }, [csrf, entryId])

  const blocks = contractToView(preview?.document?.blocks)
  // 목차는 응답에 있는 제목만 쓴다. 앵커가 없는 제목은 링크를 만들지 않는다.
  const headings = blocks.filter((block) => block.kind === 'heading' && block.anchor)

  // 세션이 없으면 초안 API를 호출하지 않고 로그인 안내만 보여 준다.
  if (!csrf) {
    return (
      <AdminFrame title="미리보기" label="공개 페이지와 같은 렌더러로 초안을 보여줍니다" current="entries">
        <p className="dg-item px-4 py-3" role="alert">로그인이 필요하다.</p>
        <p>
          <a className="dg-btn" href="/admin/login">소유자 로그인</a>
        </p>
      </AdminFrame>
    )
  }

  return (
    <AdminFrame
      title="미리보기"
      label="공개 페이지와 같은 렌더러로 초안을 보여줍니다"
      current="entries"
      action={
        <span className="flex flex-wrap gap-2">
          <button className={width === 'desktop' ? 'dg-btn' : 'dg-btn-ghost'} type="button" onClick={() => setWidth('desktop')}>
            데스크톱
          </button>
          <button className={width === 'mobile' ? 'dg-btn' : 'dg-btn-ghost'} type="button" onClick={() => setWidth('mobile')}>
            모바일
          </button>
          <a className="dg-btn-ghost" href={`/admin/entries/${entryId}/edit`}>편집으로 돌아가기</a>
        </span>
      }
    >
      <p className="dg-item mb-4 px-4 py-3">초안 미리보기 · 공개본은 바뀌지 않습니다</p>
      {message ? <p className="dg-item mb-4 px-4 py-3" role="alert">{message}</p> : null}
      {!preview && !message ? <p>미리보기를 불러오는 중이다.</p> : null}
      {preview ? (
        <div className={width === 'mobile' ? 'mx-auto max-w-[390px]' : ''}>
          <p className="dg-panel mb-4 px-4 py-3 text-sm">
            {publicPath(preview.kind, preview.slug)} · 초안 {preview.revisionId}
          </p>
          <div className="grid gap-4 min-[64rem]:grid-cols-[1fr_16rem]">
            <article className="dg-panel p-5">
              <p className="dg-label">{kindLabel(preview.kind)}</p>
              <h2 className="mb-3 text-3xl">{preview.title}</h2>
              {preview.summary ? <p className="text-quiet">{preview.summary}</p> : null}
              <BlockList blocks={blocks} />
            </article>
            <aside className="dg-panel h-fit p-4">
              <h2 className="mb-3 text-lg">이 글의 목차</h2>
              {headings.length === 0 ? <p className="mb-0 text-sm">목차에 올릴 제목이 없다.</p> : null}
              <ol className="m-0 grid list-none gap-2 p-0">
                {headings.map((heading, index) => (
                  <li key={heading.anchor}>
                    <a href={`#${heading.anchor}`}>
                      {String(index + 1).padStart(2, '0')} {heading.text}
                    </a>
                  </li>
                ))}
              </ol>
            </aside>
          </div>
        </div>
      ) : null}
    </AdminFrame>
  )
}
