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
const panel = 'mb-4 border border-rule bg-paper px-4 py-3'
const quiet = 'mb-4 text-quiet'
const card = 'border border-rule bg-paper p-4'
const stack = 'm-0 mb-4 grid list-none gap-4 p-0'

function Page(props: FrameProps) {
  return <Frame {...props} />
}

function Empty({ children }: { children: string }) {
  return <p className={panel}>{children}</p>
}

function Mark({ alt }: { alt: string }) {
  return <img src={markUrl} alt={alt} width="64" height="64" />
}

export function HomePage({ home }: { home: HomeData }) {
  const kindLabel = { note: '노트', paper: '논문', news: '뉴스' }
  const topicHref = {
    note: (slug: string) => `/notes/${slug}`,
    paper: (slug: string) => `/research/${slug}`,
    news: (slug: string) => `/research/${slug}`,
  }
  return (
    <Page title="DataGrizzly" current="/" description="프로젝트 설명과 과정 기록을 나누어 보여주는 공개 웹이다.">
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">DataGrizzly</h1>
      <p className="mb-4">프로젝트 설명과 과정 기록을 나누어 보여주는 공개 웹이다.</p>
      <p className="mb-4 flex flex-wrap gap-x-4 gap-y-2">
        <a className="text-mark" href="/about">소개 보기</a>
        <a className="text-mark" href="#featured-project">예시 프로젝트 보기</a>
      </p>
      {home.loadError ? (
        <Empty>예시 파일을 읽지 못했다. 이 페이지는 빈 준비 상태로 남는다.</Empty>
      ) : (
        <>
          <section id="featured-project">
            <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">대표 프로젝트</h2>
            {home.project ? (
              <>
                <p className={panel}>예시이며 실제 성과가 아니다.</p>
                <h3 className="mb-3 text-[1.15rem]">
                  {home.project.slug ? (
                    <a className="text-mark" href={`/projects/${home.project.slug}`}>{home.project.title}</a>
                  ) : (
                    home.project.title
                  )}
                </h3>
                <p className="mb-4">{home.project.summary}</p>
                {home.project.imageName === 'example-mark.svg' ? (
                  <Mark alt={home.project.imageAlt} />
                ) : (
                  <Empty>로컬 이미지 파일을 찾지 못했다.</Empty>
                )}
              </>
            ) : (
              <Empty>공개할 프로젝트가 아직 없다.</Empty>
            )}
          </section>
          <section>
            <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">최근 과정 기록</h2>
            {home.logs.length > 0 ? (
              <ol className="mb-4 list-decimal pl-6">
                {home.logs.map((log) => (
                  <li key={log.slug || log.title} className="mb-4">
                    {log.slug ? (
                      <p className="mb-4">
                        <a className="text-mark" href={`/logs/${log.slug}`}>{log.title}</a>
                      </p>
                    ) : (
                      <p className="mb-4">{log.title}</p>
                    )}
                    <p className={quiet}>
                      읽는 순서 {log.displayOrder + 1} · {log.stage} · 작성 시각 {log.createdAt}
                    </p>
                  </li>
                ))}
              </ol>
            ) : (
              <Empty>공개할 과정 기록이 아직 없다.</Empty>
            )}
          </section>
          <section>
            <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">노트와 리서치</h2>
            {home.topics.length > 0 ? (
              <ul className="mb-4 list-disc pl-6">
                {home.topics.map((topic) => (
                  <li key={`${topic.kind}-${topic.slug}`}>
                    {kindLabel[topic.kind]} ·{' '}
                    {topic.slug ? (
                      <a className="text-mark" href={topicHref[topic.kind](topic.slug)}>{topic.title}</a>
                    ) : (
                      topic.title
                    )}
                  </li>
                ))}
              </ul>
            ) : (
              <Empty>공개할 노트와 리서치가 아직 없다.</Empty>
            )}
          </section>
          <section>
            <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">연락</h2>
            {home.contact ? (
              <a className="text-mark" href={home.contact}>{home.contact}</a>
            ) : (
              <Empty>연락 링크는 아직 없다.</Empty>
            )}
          </section>
        </>
      )}
    </Page>
  )
}

const aboutSections = [
  { title: '지향하는 엔지니어', text: '소개 문장이 아직 없다.' },
  { title: '관심 도메인', text: '관심 도메인이 아직 없다.' },
  { title: '기술 역량', text: '근거가 되는 실제 프로젝트가 아직 없다. 홈의 예시를 경력으로 쓰지 않는다.' },
  { title: 'GitHub와 연락', text: 'GitHub 주소와 연락 링크가 아직 없다.' },
]

export function AboutPage() {
  return (
    <Page title="소개 · DataGrizzly" current="/about" description="소개 문장이 아직 없다.">
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">소개</h1>
      <p className="mb-4">이 페이지는 준비 상태다. 실제 이력은 제공된 내용만 적는다.</p>
      {aboutSections.map((section) => (
        <section key={section.title}>
          <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">{section.title}</h2>
          <Empty>{section.text}</Empty>
        </section>
      ))}
    </Page>
  )
}

export function ProjectListPage({ projects, loadError }: { projects: PublicProject[]; loadError: boolean }) {
  return (
    <Page title="프로젝트 · DataGrizzly" current="/projects" description="공개된 예시 프로젝트만 보여 준다.">
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">프로젝트</h1>
      <p className="mb-4">공개된 예시 프로젝트만 보여 준다.</p>
      <p className={quiet}>개발 중, 공개, 유지보수로 나누는 필터는 아직 없다.</p>
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
              <p className={panel}>예시이며 실제 성과가 아니다.</p>
              <p className="mb-4">{project.status || '상태가 아직 없다.'}</p>
              <Empty>주제가 아직 없다.</Empty>
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
        <a className="text-mark" href={`/projects/${slug}`} aria-current={tab === 'description' ? 'page' : undefined}>설명</a>
        <a className="text-mark" href={`/projects/${slug}/logs`} aria-current={tab === 'logs' ? 'page' : undefined}>과정</a>
      </nav>
    </header>
  )
}

export function ProjectPage({ project }: { project: PublicProject }) {
  const sections = [
    { title: '문제', text: project.problem },
    { title: '목표', text: project.goal },
    { title: '범위', text: project.scope },
    { title: '기술 선택', text: project.choice },
    { title: '판단', text: project.judgment },
    { title: '결과', text: project.result },
    { title: '한계', text: project.limit },
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
      <ProjectIdentity
        title={project.title}
        status={project.status || '상태가 아직 없다.'}
        exampleText={project.callout || '예시이며 실제 성과가 아니다.'}
        slug={project.slug}
        tab="description"
      />
      {sections.map((section) => (
        <section key={section.title}>
          <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">{section.title}</h2>
          {section.text ? <p className="mb-4">{section.text}</p> : <Empty>이 구역의 문장이 아직 없다.</Empty>}
        </section>
      ))}
      <section>
        <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">GitHub</h2>
        {typeof project.github === 'string' ? (
          <a className="text-mark" href={project.github}>{project.github}</a>
        ) : (
          <Empty>GitHub 주소가 아직 없다.</Empty>
        )}
      </section>
      <section>
        <h2 className="mb-3 mt-10 text-[1.35rem] leading-snug">관련 기록</h2>
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
      description="공개 과정을 읽는 순서로 보여 준다."
    >
      <ProjectIdentity
        title={project.title}
        status={project.status || '상태가 아직 없다.'}
        exampleText={project.callout || '예시이며 실제 성과가 아니다.'}
        slug={project.slug}
        tab="logs"
      />
      <p className={quiet}>목록은 읽는 순서다. 작성 시각은 그 순서와 다를 수 있다.</p>
      {project.logs.length > 0 ? (
        <ol className={stack}>
          {project.logs.map((log) => (
            <li key={log.slug}>
              <h2 className="mb-3 mt-0 text-[1.35rem]">
                <a className="text-mark" href={`/logs/${log.slug}`}>{log.title}</a>
              </h2>
              <p className={quiet}>
                읽는 순서 {log.displayOrder + 1} · {log.stage} · 작성 시각 {log.createdAt}
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
          <li><a className="text-mark" href={`/projects/${log.projectSlug}/logs`}>과정</a></li>
          <li>{log.title}</li>
        </ol>
      </nav>
      <p className={panel}>{log.callout || '예시이며 실제 성과가 아니다.'}</p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">{log.title}</h1>
      <p className={quiet}>읽는 순서 {log.displayOrder + 1} · {log.stage} · 작성 시각 {log.createdAt}</p>
      {log.paragraphs.map((paragraph) => (
        <p key={paragraph} className="mb-4">{paragraph}</p>
      ))}
      <nav className="mb-4 flex flex-wrap gap-x-4 gap-y-2" aria-label="인접한 과정">
        {log.previous ? (
          <a className="text-mark" href={`/logs/${log.previous.slug}`}>이전 과정 · {log.previous.title}</a>
        ) : (
          <p className="mb-4">이전 과정이 없다.</p>
        )}
        {log.next ? (
          <a className="text-mark" href={`/logs/${log.next.slug}`}>다음 과정 · {log.next.title}</a>
        ) : (
          <p className="mb-4">다음 과정이 없다.</p>
        )}
      </nav>
      <p className="mb-4">
        <a className="text-mark" href={`/projects/${log.projectSlug}`}>프로젝트 설명으로</a>
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
    <Page title="노트 · DataGrizzly" current="/notes" description="프로젝트에 속하지 않아도 되는 공개 학습 노트다.">
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">노트</h1>
      <p className="mb-4">프로젝트에 속하지 않아도 되는 공개 학습 노트다.</p>
      <p className={quiet}>언어, CS, 수학으로 나누는 필터는 아직 없다.</p>
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
              <p className={panel}>예시이며 실제 성과가 아니다.</p>
              <Empty>주제가 아직 없다.</Empty>
              <p className="mb-4">{note.summary}</p>
              <p className={quiet}>작성 시각 {note.createdAt}</p>
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
      <p className="mb-4"><a className="text-mark" href="/notes">노트 목록으로</a></p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">{article.title}</h1>
      <p className={quiet}>작성 시각 {article.createdAt}</p>
      {article.projectId ? <p className={quiet}>프로젝트 {article.projectId}</p> : <Empty>이 글은 프로젝트에 속하지 않는다.</Empty>}
      {article.blocks.length > 0 ? <BlockList blocks={article.blocks} /> : <Empty>본문이 아직 없다.</Empty>}
    </Page>
  )
}

export function ResearchListPage({ articles, loadError }: { articles: PublicArticle[]; loadError: boolean }) {
  return (
    <Page title="리서치 · DataGrizzly" current="/research" description="논문과 뉴스를 같은 경로에서 종류로 구분한다.">
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">리서치</h1>
      <p className="mb-4">논문과 뉴스를 같은 경로에서 종류로 구분한다.</p>
      <p className={quiet}>논문과 뉴스를 거르는 필터는 아직 없다.</p>
      {loadError ? (
        <Empty>예시 파일을 읽지 못했다. 이 페이지는 빈 목록으로 남는다.</Empty>
      ) : articles.length === 0 ? (
        <Empty>공개 리서치가 아직 없다.</Empty>
      ) : (
        <ul className={stack}>
          {articles.map((article) => (
            <li key={article.slug} className={card}>
              <p className={quiet}>{article.kind === 'paper' ? '논문' : '뉴스'}</p>
              <h2 className="mb-3 mt-0 text-[1.35rem]">
                <a className="text-mark" href={`/research/${article.slug}`}>{article.title}</a>
              </h2>
              <p className={panel}>예시이며 실제 성과가 아니다.</p>
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
  const kindLabel = article.kind === 'paper' ? '논문' : '뉴스'
  return (
    <Page
      title={`${article.title} · DataGrizzly`}
      current="/research"
      canonical={`/research/${article.slug}`}
      description={article.summary || '리서치 본문이 아직 없다.'}
      date={article.updatedAt}
      dateKind="modified"
    >
      <p className="mb-4"><a className="text-mark" href="/research">리서치 목록으로</a></p>
      <p className={quiet}>{kindLabel}</p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">{article.title}</h1>
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
      <p className={panel}>예시 분류이며 실제 성과가 아니다.</p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">{topic.title}</h1>
      <Empty>관련 키워드는 아직 없다.</Empty>
      <ul className="mb-4 list-disc pl-6">
        {topic.entries.map((entry) => (
          <li key={entry.path}>
            <a className="text-mark" href={entry.path}>{entry.title}</a>
          </li>
        ))}
      </ul>
      <p className={quiet}>언어, CS, 수학으로 나누는 필터는 아직 없다.</p>
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
      <p className={panel}>예시 분류이며 실제 성과가 아니다.</p>
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">{series.title}</h1>
      <Empty>학습 목표는 아직 없다.</Empty>
      <ol className="mb-4 list-decimal pl-6">
        {series.entries.map((entry, index) => (
          <li key={entry.path}>
            <a className="text-mark" href={entry.path}>{entry.title}</a>
            <span className="text-quiet"> 순서 {index + 1}</span>
          </li>
        ))}
      </ol>
    </Page>
  )
}

export function MissingPage() {
  return (
    <Page
      title="페이지를 찾을 수 없다 · DataGrizzly"
      current={null}
      includeCanonical={false}
      noindex={true}
      description="이 주소의 공개 글은 없다."
    >
      <h1 className="mb-4 text-[length:var(--font-size-title)] leading-tight">페이지를 찾을 수 없다</h1>
      <p className="mb-4">이 주소의 공개 글은 없다.</p>
      <Empty>검색은 아직 연결하지 않았다. 검색어가 없는 결과 페이지도 만들지 않는다.</Empty>
      <p className="mb-4">공개 글 읽기는 아래 목록에서 계속할 수 있다.</p>
      <p className="mb-4 flex flex-wrap gap-x-4 gap-y-2">
        <a className="text-mark" href="/">홈</a>
        <a className="text-mark" href="/projects">프로젝트</a>
        <a className="text-mark" href="/notes">노트</a>
        <a className="text-mark" href="/research">리서치</a>
        <a className="text-mark" href="/about">소개</a>
      </p>
    </Page>
  )
}
