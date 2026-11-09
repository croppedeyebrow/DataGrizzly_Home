/**
 * 개념: 공개 경로의 React 페이지.
 * 목적: Astro 페이지와 같은 문장, 주소, 빈 상태를 정적 HTML로 남긴다.
 * 사용처: apps/web/src/prerender.ts
 */
import type { PublicArticle } from '../scripts/research-types'
import type { PublicLog, PublicProject } from '../scripts/project-types'
import type { TaxonomyPage } from '../scripts/taxonomy-types'
import { BlockList } from './blocks'
import type { HomeData } from './home'
import { Frame, type FrameProps } from './layout'

const markUrl = '/media/example-mark.svg'
const panel = 'dg-item mb-4 px-4 py-3'
const quiet = 'mb-4 text-quiet'
const card = 'dg-panel p-5'
const stack = 'm-0 mb-6 grid list-none gap-4 p-0'

function Page(props: FrameProps) {
  return <Frame {...props} />
}

function Empty({ children }: { children: string }) {
  return <p className={panel}>{children}</p>
}

function Mark({ alt }: { alt: string }) {
  return <img src={markUrl} alt={alt} width="64" height="64" />
}

const topicTiles = [
  { kind: 'note' as const, label: '노트', fallback: '/notes' },
  { kind: 'paper' as const, label: '논문 리뷰', fallback: '/research' },
  { kind: 'news' as const, label: '뉴스 정리', fallback: '/research' },
]
const topicHref = {
  note: (slug: string) => `/notes/${slug}`,
  paper: (slug: string) => `/research/${slug}`,
  news: (slug: string) => `/research/${slug}`,
}
const waveHeights = [8, 16, 10, 20, 12, 18, 9, 22, 11, 16, 8, 14]

function Wave() {
  return (
    <div className="mb-5 flex h-6 items-end gap-1" aria-hidden="true">
      {waveHeights.map((height, index) => (
        <span
          key={index}
          className="w-1 rounded-full bg-[var(--color-accent)]"
          style={{ height }}
        />
      ))}
    </div>
  )
}

export function HomePage({ home }: { home: HomeData }) {
  const logsHref = home.project?.slug ? `/projects/${home.project.slug}/logs` : '/projects'
  return (
    <Page title="DataGrizzly" current="/" description="문제를 정의하고, 판단의 근거를 남기고, 구현과 검증까지 한 흐름으로 기록합니다.">
      <section className="dg-hero mb-4 grid items-center gap-8 px-6 py-8 min-[64rem]:grid-cols-[minmax(0,1.15fr)_minmax(16rem,22rem)] min-[64rem]:px-10">
        <div>
          <p className="dg-chip mb-5">DATAGRIZZLY · 기록하는 엔지니어링</p>
          <h1 className="mb-5 max-w-3xl text-4xl font-bold leading-[1.25] min-[48rem]:text-[3.25rem]">
            <span className="block">현장의 데이터에서,</span>
            <span className="dg-text-gradient block">신뢰할 수 있는 시스템까지</span>
          </h1>
          <Wave />
          <p className="keep-ko mb-6 max-w-xl text-quiet">문제를 정의하고, 판단의 근거를 남기고, 구현과 검증까지 한 흐름으로 기록합니다.</p>
          <p className="m-0 flex flex-wrap gap-3">
            <a className="dg-btn" href="/projects">프로젝트 보기 →</a>
            <a className="dg-btn-ghost" href="/notes">노트 읽기</a>
          </p>
        </div>
        <img className="mx-auto w-full max-w-sm" src="/media/grizzly-researcher.png" alt="DataGrizzly 표지" />
      </section>
      {home.loadError ? (
        <Empty>예시 파일을 읽지 못했다. 이 페이지는 빈 준비 상태로 남는다.</Empty>
      ) : (
        <>
          <div className="grid gap-4 min-[64rem]:grid-cols-3">
            <section id="featured-project" className="dg-panel p-5">
              <p className="dg-label mb-2">대표 프로젝트</p>
              <h2 className="mb-1 mt-0 text-xl leading-snug">대표 프로젝트 · 예시 콘텐츠</h2>
              <p className="mb-4 text-sm text-quiet">문제 · 판단 · 구현 · 검증</p>
              {home.project ? (
                <a className="dg-item flex items-center gap-3 px-3 py-3 no-underline" href={home.project.slug ? `/projects/${home.project.slug}` : undefined}>
                  <span className="dg-icon-tile size-10 shrink-0 text-sm" aria-hidden="true">P</span>
                  <span className="min-w-0">
                    <span className="block text-ink">{home.project.title}</span>
                    <span className="block text-sm text-quiet">{home.project.summary}</span>
                  </span>
                </a>
              ) : (
                <Empty>공개할 프로젝트가 아직 없다.</Empty>
              )}
              {home.project && home.project.imageName !== 'example-mark.svg' ? (
                <Empty>로컬 이미지 파일을 찾지 못했다.</Empty>
              ) : null}
            </section>
            <section className="dg-panel p-5">
              <p className="dg-label mb-2">과정 기록</p>
              <h2 className="mb-1 mt-0 text-xl leading-snug">최근 과정 기록</h2>
              <p className="mb-4 text-sm text-quiet">문제 · 판단 · 구현 · 검증</p>
              {home.logs.length > 0 ? (
                <ol className="m-0 grid list-none gap-3 p-0">
                  {home.logs.map((log) => (
                    <li key={log.slug || log.title}>
                      <a className="dg-item flex items-center gap-3 px-3 py-3 no-underline" href={log.slug ? `/logs/${log.slug}` : undefined}>
                        <span className="dg-orb inline-block size-2 shrink-0" aria-hidden="true" />
                        <span className="min-w-0">
                          <span className="block text-ink">{log.title}</span>
                          <span className="block text-sm text-quiet">
                            {log.createdAt.slice(0, 10)} · {home.project?.title || log.stage}
                          </span>
                        </span>
                      </a>
                    </li>
                  ))}
                </ol>
              ) : (
                <Empty>공개할 과정 기록이 아직 없다.</Empty>
              )}
            </section>
            <section className="dg-panel p-5">
              <p className="dg-label mb-2">주제별</p>
              <h2 className="mb-1 mt-0 text-xl leading-snug">주제별 노트 · 리서치</h2>
              <p className="mb-4 text-sm text-quiet">문제 · 판단 · 구현 · 검증</p>
              {home.topics.length > 0 ? (
                <ul className="m-0 grid list-none grid-cols-3 gap-3 p-0">
                  {topicTiles.map((tile) => {
                    const topic = home.topics.find((item) => item.kind === tile.kind)
                    const href = topic?.slug ? topicHref[tile.kind](topic.slug) : tile.fallback
                    return (
                      <li key={tile.kind}>
                        <a className="dg-item flex h-full flex-col items-center justify-center gap-2 px-2 py-4 text-center no-underline" href={href}>
                          <span className="dg-icon-tile size-10 text-xs" aria-hidden="true">{tile.label.slice(0, 1)}</span>
                          <span className="text-sm text-ink">{tile.label}</span>
                        </a>
                      </li>
                    )
                  })}
                </ul>
              ) : (
                <Empty>공개할 노트와 리서치가 아직 없다.</Empty>
              )}
            </section>
          </div>
          <div className="dg-panel mt-4 flex flex-wrap items-center gap-3 px-4 py-3">
            <span className="dg-label">읽는 흐름</span>
            <a className="dg-chip no-underline" href={logsHref}>최근 과정 기록</a>
            <span aria-hidden="true">→</span>
            <a className="dg-chip no-underline" href="/notes">학습 노트</a>
            <span aria-hidden="true">→</span>
            <a className="dg-chip no-underline" href="/research">근거 자료</a>
          </div>
          {home.contact ? <p className="mb-0 mt-4 text-sm text-quiet"><a href={home.contact}>{home.contact}</a></p> : null}
        </>
      )}
    </Page>
  )
}

const aboutSections = [
  { title: '지향하는 엔지니어', text: '어떤 문제를, 어떤 방식으로 풀고 싶은지 적습니다.' },
  { title: '관심 도메인 · 산업 데이터와 시스템', text: '도메인을 적습니다.' },
  { title: '기술 역량 · 근거 프로젝트', text: '근거 프로젝트가 곧 기술 역량입니다. 없는 성과는 적지 않습니다.' },
  { title: 'GitHub · 연락하기', text: 'GitHub 보기 · 이메일 보내기' },
]

export function AboutPage() {
  return (
    <Page title="소개 · DataGrizzly" current="/about" description="현장의 데이터를 믿을 수 있는 시스템으로">
      <p className="dg-label mb-3">소개</p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">현장의 데이터를 믿을 수 있는 시스템으로</h1>
      <p className="mb-4 text-quiet">DataGrizzly</p>
      <p className="dg-label mb-4">이 글의 목차</p>
      {aboutSections.map((section, index) => (
        <section key={section.title}>
          <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">{String(index + 1).padStart(2, '0')} {section.title}</h2>
          <Empty>{section.text}</Empty>
        </section>
      ))}
    </Page>
  )
}

export function ProjectListPage({ projects, loadError }: { projects: PublicProject[]; loadError: boolean }) {
  return (
    <Page title="프로젝트 · DataGrizzly" current="/projects" description="문제를 정의하고 구현과 검증까지 기록한 프로젝트 모음">
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">프로젝트</h1>
      <p className="mb-4">문제를 정의하고 구현과 검증까지 기록한 프로젝트 모음</p>
      <p className="mb-4 text-quiet">전체 · 개발 중 · 공개 · 유지보수 · {projects.length}개 프로젝트</p>
      {loadError ? (
        <Empty>예시 파일을 읽지 못했다. 이 페이지는 빈 목록으로 남는다.</Empty>
      ) : projects.length === 0 ? (
        <Empty>공개 프로젝트가 아직 없다.</Empty>
      ) : (
        <ul className={stack}>
          {projects.map((project) => (
            <li key={project.slug} className={card}>
              <h2 className="mb-3 mt-0 text-[1.35rem]">
                <a className="text-mark" href={`/projects/${project.slug}`}>{project.title}</a>
              </h2>
              <p className="mb-4">문제 · 기술 · 진행 상태{project.status ? ` · ${project.status}` : ''}</p>
              <p className="mb-4">{project.summary}</p>
              <p className={quiet}>갱신 {project.updatedAt}</p>
            </li>
          ))}
        </ul>
      )}
    </Page>
  )
}

function ProjectIdentity({
  title,
  status,
  exampleText,
  slug,
  tab,
}: {
  title: string
  status: string
  exampleText: string
  slug: string
  tab: 'description' | 'logs'
}) {
  return (
    <header>
      <p className={panel}>{exampleText}</p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">{title}</h1>
      <p className="mb-4">{status}</p>
      <nav className="project-tabs mb-4 flex flex-wrap gap-x-4 gap-y-2" aria-label="프로젝트 구역">
        <a className="text-mark" href={`/projects/${slug}`} aria-current={tab === 'description' ? 'page' : undefined}>프로젝트 설명</a>
        <a className="text-mark" href={`/projects/${slug}/logs`} aria-current={tab === 'logs' ? 'page' : undefined}>과정 기록</a>
      </nav>
    </header>
  )
}

export function ProjectPage({ project }: { project: PublicProject }) {
  const sections = [
    { title: '해결하려는 문제 · 목표와 범위', texts: [project.problem, project.goal, project.scope] },
    { title: '구조와 기술 선택', texts: [project.choice, project.judgment] },
    { title: '담당 범위 · 검증 결과', texts: [project.result] },
    { title: '한계 · GitHub · 관련 기록', texts: [project.limit] },
  ]
  const relatedLogs = project.related.filter((item) => item.kind === 'log' && item.href)
  return (
    <Page
      title={`${project.title} · DataGrizzly`}
      current="/projects"
      canonical={`/projects/${project.slug}`}
      description={project.summary || '프로젝트 설명 문장이 아직 없다.'}
      date={project.updatedAt}
      dateKind="modified"
    >
      <p className="mb-4 text-quiet">프로젝트 / {project.title}</p>
      <ProjectIdentity
        title={project.title}
        status={project.status || '문제 · 기술 · 진행 상태'}
        exampleText={project.callout || '예시이며 실제 성과가 아니다.'}
        slug={project.slug}
        tab="description"
      />
      {sections.map((section) => (
        <section key={section.title}>
          <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">{section.title}</h2>
          {section.texts.map((text, index) => (
            text ? <p key={index} className="mb-4">{text}</p> : <Empty key={index}>이 구역의 문장이 아직 없다.</Empty>
          ))}
        </section>
      ))}
      <section>
        <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">GitHub 저장소</h2>
        {typeof project.github === 'string' ? (
          <a className="text-mark" href={project.github}>{project.github}</a>
        ) : (
          <Empty>GitHub 주소가 아직 없다.</Empty>
        )}
      </section>
      <section>
        <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">과정 기록 읽기</h2>
        {relatedLogs.length > 0 ? (
          <ul className="mb-4 list-disc pl-6">
            {relatedLogs.map((item) => (
              <li key={item.href ?? item.title}>
                <a className="text-mark" href={item.href ?? ''}>{item.title}</a>
              </li>
            ))}
          </ul>
        ) : (
          <Empty>관련 과정 기록이 아직 없다.</Empty>
        )}
      </section>
      {project.imageName === 'example-mark.svg' ? <Mark alt={project.imageAlt} /> : <Empty>로컬 이미지 파일을 찾지 못했다.</Empty>}
    </Page>
  )
}

export function ProjectLogsPage({ project }: { project: PublicProject }) {
  return (
    <Page
      title={`과정 · ${project.title}`}
      current="/projects"
      canonical={`/projects/${project.slug}/logs`}
      description="읽는 순서와 작성일은 별개입니다."
    >
      <p className="mb-4 text-quiet">프로젝트 / {project.title}</p>
      <ProjectIdentity
        title={project.title}
        status={project.status || '상태가 아직 없다.'}
        exampleText={project.callout || '예시이며 실제 성과가 아니다.'}
        slug={project.slug}
        tab="logs"
      />
      <p className={quiet}>읽는 순서와 작성일은 별개입니다.</p>
      {project.logs.length > 0 ? (
        <ol className={stack}>
          {project.logs.map((log) => (
            <li key={log.slug}>
              <h2 className="mb-3 mt-0 text-[1.35rem]">
                <a className="text-mark" href={`/logs/${log.slug}`}>{log.title}</a>
              </h2>
              <p className={quiet}>
                {log.stage} · {log.createdAt.slice(0, 10)} · 읽기 →
              </p>
            </li>
          ))}
        </ol>
      ) : (
        <Empty>공개된 과정 기록이 아직 없다.</Empty>
      )}
    </Page>
  )
}

export function LogPage({ log }: { log: PublicLog }) {
  const notes = log.related.filter((item) => item.kind === 'note' || item.kind === 'paper' || item.kind === 'news')
  return (
    <Page
      title={`${log.title} · DataGrizzly`}
      current="/projects"
      canonical={`/logs/${log.slug}`}
      description={log.paragraphs[0] || '과정 기록 문장이 아직 없다.'}
      date={log.createdAt}
      dateKind="published"
    >
      <nav aria-label="현재 위치">
        <ol className="mb-4 flex list-none flex-wrap gap-x-4 gap-y-2 p-0">
          <li><a className="text-mark" href="/projects">프로젝트</a></li>
          <li><a className="text-mark" href={`/projects/${log.projectSlug}`}>{log.projectTitle}</a></li>
          <li><a className="text-mark" href={`/projects/${log.projectSlug}/logs`}>과정 기록</a></li>
          <li>{log.title}</li>
        </ol>
      </nav>
      <p className={panel}>{log.callout || '예시이며 실제 성과가 아니다.'}</p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">{log.title}</h1>
      <p className={quiet}>구현 단계 {log.stage} · DataGrizzly · {log.createdAt.slice(0, 10)}</p>
      {log.paragraphs.map((paragraph) => (
        <p key={paragraph} className="mb-4">{paragraph}</p>
      ))}
      <nav className="mb-4 flex flex-wrap gap-x-4 gap-y-2" aria-label="인접한 과정">
        {log.previous ? (
          <a className="text-mark" href={`/logs/${log.previous.slug}`}>← 이전 과정 · {log.previous.title}</a>
        ) : (
          <p className="mb-4">이전 과정이 없다.</p>
        )}
        {log.next ? (
          <a className="text-mark" href={`/logs/${log.next.slug}`}>다음 과정 → · {log.next.title}</a>
        ) : (
          <p className="mb-4">다음 과정이 없다.</p>
        )}
      </nav>
      <p className="mb-4">
        <a className="text-mark" href={`/projects/${log.projectSlug}`}>프로젝트로 · {log.projectTitle}</a>
      </p>
      <section>
        <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">관련 노트</h2>
        {notes.length > 0 ? (
          notes.map((note) => (
            <p key={note.title} className="mb-4">{note.title}. 상세 페이지는 아직 없다.</p>
          ))
        ) : (
          <Empty>관련 노트가 아직 없다.</Empty>
        )}
      </section>
    </Page>
  )
}

export function NoteListPage({ notes, loadError }: { notes: PublicArticle[]; loadError: boolean }) {
  return (
    <Page title="노트 · DataGrizzly" current="/notes" description="공부하며 정리한 개념과 예제">
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">노트</h1>
      <p className="mb-4">공부하며 정리한 개념과 예제</p>
      <p className="mb-4 text-quiet">전체 · 언어 · CS · AI·ML · 수학 · 물리·화학</p>
      <p className="dg-label mb-4">주제와 시리즈로 탐색</p>
      {loadError ? (
        <Empty>예시 파일을 읽지 못했다. 이 페이지는 빈 목록으로 남는다.</Empty>
      ) : notes.length === 0 ? (
        <Empty>공개 노트가 아직 없다.</Empty>
      ) : (
        <ul className={stack}>
          {notes.map((note) => (
            <li key={note.slug} className={card}>
              <h2 className="mb-3 mt-0 text-[1.35rem]">
                <a className="text-mark" href={`/notes/${note.slug}`}>{note.title}</a>
              </h2>
              <p className="mb-4">{note.summary}</p>
              <p className={quiet}>{note.createdAt.slice(0, 10)} · 상세 보기 →</p>
            </li>
          ))}
        </ul>
      )}
    </Page>
  )
}

export function NotePage({ article }: { article: PublicArticle }) {
  return (
    <Page
      title={`${article.title} · DataGrizzly`}
      current="/notes"
      canonical={`/notes/${article.slug}`}
      description={article.summary || '노트 본문이 아직 없다.'}
      date={article.updatedAt}
      dateKind="modified"
    >
      <p className="mb-4"><a className="text-mark" href="/notes">노트</a></p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">{article.title}</h1>
      <p className={quiet}>DataGrizzly · {article.createdAt.slice(0, 10)}</p>
      <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">학습 목표 · 핵심 질문</h2>
      {article.summary ? <p className="mb-4">{article.summary}</p> : <Empty>이 노트로 답하려는 질문을 적습니다.</Empty>}
      <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">개념 설명 · 코드 · 수식</h2>
      {article.projectId ? <p className={quiet}>프로젝트 {article.projectId}</p> : <Empty>이 글은 프로젝트에 속하지 않는다.</Empty>}
      {article.blocks.length > 0 ? <BlockList blocks={article.blocks} /> : <Empty>본문이 아직 없다.</Empty>}
    </Page>
  )
}

export function ResearchListPage({ articles, loadError }: { articles: PublicArticle[]; loadError: boolean }) {
  return (
    <Page title="리서치 · DataGrizzly" current="/research" description="논문 리뷰와 도메인 뉴스 정리 · 원문 출처를 함께 표시합니다">
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">리서치</h1>
      <p className="mb-4">논문 리뷰와 도메인 뉴스 정리 · 원문 출처를 함께 표시합니다</p>
      <p className="mb-4 text-quiet">전체 · 논문 리뷰 · 도메인 뉴스</p>
      {loadError ? (
        <Empty>예시 파일을 읽지 못했다. 이 페이지는 빈 목록으로 남는다.</Empty>
      ) : articles.length === 0 ? (
        <Empty>공개 리서치가 아직 없다.</Empty>
      ) : (
        <ul className={stack}>
          {articles.map((article) => (
            <li key={article.slug} className={card}>
              <p className={quiet}>{article.kind === 'paper' ? '논문 리뷰' : '도메인 뉴스'}</p>
              <h2 className="mb-3 mt-0 text-[1.35rem]">
                <a className="text-mark" href={`/research/${article.slug}`}>{article.title}</a>
              </h2>
              <p className="mb-4">원문 출처 · {article.source || article.publicationDate || article.eventDate}</p>
              {article.kind === 'paper' ? (
                <>
                  <p className="mb-4">{article.claim}</p>
                  <p className="mb-4">{article.reproduction}</p>
                </>
              ) : (
                <>
                  <p className="mb-4">{article.eventDate}</p>
                  <p className="mb-4">{article.publicationDate}</p>
                </>
              )}
            </li>
          ))}
        </ul>
      )}
    </Page>
  )
}

export function ResearchPage({ article }: { article: PublicArticle }) {
  const kindLabel = article.kind === 'paper' ? '논문 리뷰' : '도메인 뉴스'
  const sectionTitle = article.kind === 'paper' ? '문제 정의 · 방법 · 저자의 주장' : '확인된 사실'
  return (
    <Page
      title={`${article.title} · DataGrizzly`}
      current="/research"
      canonical={`/research/${article.slug}`}
      description={article.summary || '리서치 본문이 아직 없다.'}
      date={article.updatedAt}
      dateKind="modified"
    >
      <p className="mb-4"><a className="text-mark" href="/research">리서치</a> / {kindLabel}</p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">{article.title}</h1>
      <p className={quiet}>DataGrizzly · {article.updatedAt.slice(0, 10)}</p>
      {article.source ? <p className="mb-4"><a className="dg-btn" href={article.source}>원문 열기</a></p> : null}
      <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">{sectionTitle}</h2>
      {article.blocks.length > 0 ? <BlockList blocks={article.blocks} /> : <Empty>본문이 아직 없다.</Empty>}
    </Page>
  )
}

export function TopicPage({ topic }: { topic: TaxonomyPage }) {
  return (
    <Page
      title={`${topic.title} · DataGrizzly`}
      current={null}
      canonical={`/topics/${topic.slug}`}
      description={`${topic.title}에 연결된 공개 글이다.`}
    >
      <p className="dg-label mb-3">주제</p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">{topic.title}</h1>
      <p className="mb-4 text-quiet">주제 소개 한두 문장</p>
      <p className="mb-4 text-quiet">전체 · 노트 · 과정 기록 · 리서치</p>
      <p className="dg-label mb-4">다른 주제로 이동</p>
      <ul className="mb-4 list-disc pl-6">
        {topic.entries.map((entry) => (
          <li key={entry.path}>
            <a className="text-mark" href={entry.path}>{entry.title}</a>
            <span className="text-quiet"> 상세 보기 →</span>
          </li>
        ))}
      </ul>
      <p className={quiet}>관련 키워드는 아직 없다.</p>
    </Page>
  )
}

export function SeriesPage({ series }: { series: TaxonomyPage }) {
  return (
    <Page
      title={`${series.title} · DataGrizzly`}
      current={null}
      canonical={`/series/${series.slug}`}
      description={`${series.title}의 공개 글을 지정된 순서로 보여 준다.`}
    >
      <p className="dg-label mb-3">시리즈</p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">{series.title}</h1>
      <p className="mb-4 text-quiet">시리즈 소개 · 학습 목표 한 줄 · 공개 글 {series.entries.length}편</p>
      <p className="mb-4 text-quiet">시리즈 소개 · 순서대로 읽기</p>
      <p className={quiet}>공개된 글만 순서대로 표시합니다.</p>
      <ol className="mb-4 list-decimal pl-6">
        {series.entries.map((entry, index) => (
          <li key={entry.path}>
            <a className="text-mark" href={entry.path}>{entry.title}</a>
            <span className="text-quiet"> 읽기 → · 순서 {index + 1}</span>
          </li>
        ))}
      </ol>
    </Page>
  )
}

export function ExplorePage() {
  return (
    <Page title="탐색 · DataGrizzly" current="/explore" description="검색어를 입력하세요">
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">무엇을 찾고 있나요?</h1>
      <p className="dg-item mb-4 px-4 py-3">검색어를 입력하세요</p>
      <p className="mb-4 text-quiet">전체 · 프로젝트 · 과정 기록 · 노트 · 리서치</p>
      <h2 className="mb-3 text-[1.35rem]">검색 결과가 없어요</h2>
      <p className="mb-4">다른 검색어를 쓰거나 유형 필터를 ‘전체’로 바꿔 보세요.</p>
      <p className="dg-label">주제로 좁히기</p>
    </Page>
  )
}

export function MissingPage() {
  return (
    <Page
      title="페이지를 찾을 수 없어요 · DataGrizzly"
      current={null}
      includeCanonical={false}
      noindex={true}
      description="주소가 바뀌었거나 비공개로 전환된 글일 수 있습니다."
    >
      <p className="dg-label mb-2">404</p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">페이지를 찾을 수 없어요</h1>
      <p className="mb-4">주소가 바뀌었거나 비공개로 전환된 글일 수 있습니다.</p>
      <p className="mb-4 flex flex-wrap gap-3">
        <a className="dg-btn" href="/">홈으로</a>
        <a className="dg-btn-ghost" href="/explore">검색하기</a>
      </p>
      <section className="dg-panel mb-4 p-5">
        <h2 className="mb-3 mt-0 text-xl">검색 결과가 없어요</h2>
        <p className="mb-0">다른 검색어를 쓰거나 유형 필터를 ‘전체’로 바꿔 보세요.</p>
      </section>
      <section className="dg-panel mb-4 p-5">
        <h2 className="mb-3 mt-0 text-xl">검색이 잠시 멈췄어요</h2>
        <p className="mb-0">색인을 갱신하는 중입니다. 잠시 후 다시 시도해 주세요.</p>
      </section>
      <section className="dg-panel p-5">
        <h2 className="mb-3 mt-0 text-xl">공개 글은 계속 읽을 수 있어요</h2>
        <p className="mb-4">검색이 멈춰도 프로젝트·노트·리서치 목록은 그대로 열립니다.</p>
        <p className="mb-0 flex flex-wrap gap-x-4 gap-y-2">
          <a className="text-mark" href="/projects">프로젝트</a>
          <a className="text-mark" href="/notes">노트 목록 보기</a>
          <a className="text-mark" href="/research">리서치</a>
        </p>
      </section>
    </Page>
  )
}
