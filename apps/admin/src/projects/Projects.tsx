/**
 * 개념: 프로젝트 관리 화면.
 * 목적: 설명, 과정 연결, 과정 순서를 서버에 있는 값으로만 보여 준다.
 * 사용처: /admin/projects, /admin/projects/{id}.
 */
import { useEffect, useState, type FormEvent } from 'react'
import { AdminFrame } from '../AdminFrame'
import { adminJson } from '../api'
import { readCsrf } from '../session'

type ProjectCard = {
  id: string
  slug: string
  title: string
  summary: string
  status: string
  version: number
  logCount: number
  updatedAt: string
}

type ProjectLog = {
  id: string
  slug: string
  title: string
  stage: string
  displayOrder: number
  commitUrl: string
  version: number
}

type ProjectDetail = ProjectCard & { logs: ProjectLog[] }

const statuses = ['공개', '개발 중', '유지보수']

function shortDate(value: string): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})/.exec(value)
  return match ? `${match[2]}.${match[3]}` : value
}

function signedOut(title: string) {
  return (
    <AdminFrame title={title} label="설명 편집 · 과정 연결 · 과정 순서 변경" current="projects">
      <p className="dg-item px-4 py-3" role="alert">로그인이 필요하다.</p>
      <p>
        <a className="dg-btn" href="/admin/login">소유자 로그인</a>
      </p>
    </AdminFrame>
  )
}

/**
 * 개념: 프로젝트 카드 목록.
 * 목적: 비어 있거나 실패한 목록을 숨기지 않는다.
 * 사용처: App의 projects 경로.
 */
export function ProjectList() {
  const csrf = readCsrf()
  const [projects, setProjects] = useState<ProjectCard[] | null>(null)
  const [message, setMessage] = useState('')
  const [open, setOpen] = useState(false)
  const [slug, setSlug] = useState('')
  const [title, setTitle] = useState('')
  const [summary, setSummary] = useState('')
  const [status, setStatus] = useState('개발 중')
  const [pending, setPending] = useState(false)

  useEffect(() => {
    if (!csrf) return
    let cancelled = false
    async function load() {
      const result = await adminJson<{ projects?: ProjectCard[] }>('/api/admin/projects')
      if (cancelled || result.status === 401) return
      if (result.status !== 200 || !result.body?.projects) {
        setMessage(result.status === 0 ? '서버에 연결하지 못했다.' : '목록을 불러오지 못했다.')
        return
      }
      setProjects(result.body.projects)
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
    const result = await adminJson<{ id?: string }>('/api/admin/projects', {
      method: 'POST',
      body: JSON.stringify({ slug, title, summary, status }),
    })
    setPending(false)
    if (result.status === 422) {
      setMessage('슬러그, 이름, 상태를 다시 확인해 달라.')
      return
    }
    if (result.status === 409) {
      setMessage('같은 슬러그의 프로젝트가 이미 있다.')
      return
    }
    if (result.status !== 201 || !result.body?.id) {
      setMessage(result.status === 0 ? '서버에 연결하지 못했다.' : '프로젝트를 만들지 못했다.')
      return
    }
    window.location.assign(`/admin/projects/${result.body.id}`)
  }

  if (!csrf) return signedOut('프로젝트 관리')

  return (
    <AdminFrame
      title="프로젝트 관리"
      label="설명 편집 · 과정 연결 · 과정 순서 변경"
      current="projects"
      action={
        <button className="dg-btn" type="button" onClick={() => setOpen((value) => !value)}>
          + 프로젝트 생성
        </button>
      }
    >
      {open ? (
        <form className="dg-panel mb-4 grid gap-3 p-5" onSubmit={onCreate}>
          <label htmlFor="project-title">프로젝트 이름</label>
          <input className="dg-input" id="project-title" value={title} onChange={(event) => setTitle(event.target.value)} />
          <label htmlFor="project-slug">슬러그</label>
          <input className="dg-input" id="project-slug" value={slug} onChange={(event) => setSlug(event.target.value)} />
          <label htmlFor="project-summary">한 줄 설명</label>
          <input className="dg-input" id="project-summary" value={summary} onChange={(event) => setSummary(event.target.value)} />
          <label htmlFor="project-status">상태</label>
          <select className="dg-input" id="project-status" value={status} onChange={(event) => setStatus(event.target.value)}>
            {statuses.map((item) => (
              <option key={item} value={item}>{item}</option>
            ))}
          </select>
          <button className="dg-btn" type="submit" disabled={pending}>프로젝트 생성</button>
        </form>
      ) : null}
      {message ? <p className="dg-item px-4 py-3" role="alert">{message}</p> : null}
      {projects === null && !message ? <p>목록을 불러오는 중이다.</p> : null}
      {projects && projects.length === 0 ? <p className="dg-item px-4 py-3">프로젝트가 없다.</p> : null}
      {projects && projects.length > 0 ? (
        <ul className="m-0 grid list-none gap-4 p-0 min-[48rem]:grid-cols-2">
          {projects.map((project) => (
            <li key={project.id} className="dg-item grid gap-3 p-4">
              <div className="flex items-start justify-between gap-3">
                <h2 className="mb-0 text-xl">{project.title}</h2>
                <span className="dg-chip">{project.status}</span>
              </div>
              <div className="grid gap-2 min-[24rem]:grid-cols-2">
                <p className="dg-panel mb-0 px-3 py-2 text-sm">과정 수 {project.logCount}</p>
                <p className="dg-panel mb-0 px-3 py-2 text-sm">수정일 {shortDate(project.updatedAt)}</p>
              </div>
              <div className="flex flex-wrap gap-2">
                <a className="dg-btn-ghost" href={`/admin/projects/${project.id}`}>설명 편집</a>
                <a className="dg-btn-ghost" href={`/admin/projects/${project.id}#logs`}>과정 순서 변경</a>
              </div>
            </li>
          ))}
        </ul>
      ) : null}
    </AdminFrame>
  )
}

/**
 * 개념: 프로젝트 설명과 과정 기록.
 * 목적: 없는 프로젝트 연결을 거절하고, 순서 변경은 한 번의 저장으로 보낸다.
 * 사용처: /admin/projects/{id}.
 */
export function ProjectEditor({ projectId }: { projectId: string }) {
  const csrf = readCsrf()
  const [project, setProject] = useState<ProjectDetail | null>(null)
  const [message, setMessage] = useState('')
  const [pending, setPending] = useState(false)
  const [title, setTitle] = useState('')
  const [summary, setSummary] = useState('')
  const [status, setStatus] = useState('개발 중')
  const [version, setVersion] = useState(1)
  const [logs, setLogs] = useState<ProjectLog[]>([])
  const [logSlug, setLogSlug] = useState('')
  const [logTitle, setLogTitle] = useState('')
  const [stage, setStage] = useState('')
  const [order, setOrder] = useState(1)
  const [commitUrl, setCommitUrl] = useState('')

  async function load() {
    const result = await adminJson<ProjectDetail>(`/api/admin/projects/${projectId}`)
    if (result.status === 401) return
    if (result.status === 404) {
      setMessage('프로젝트가 없다.')
      return
    }
    if (result.status !== 200 || !result.body) {
      setMessage(result.status === 0 ? '서버에 연결하지 못했다.' : '프로젝트를 불러오지 못했다.')
      return
    }
    setProject(result.body)
    setTitle(result.body.title)
    setSummary(result.body.summary)
    setStatus(result.body.status)
    setVersion(result.body.version)
    setLogs(result.body.logs)
  }

  useEffect(() => {
    if (!csrf) return
    void load()
  }, [csrf, projectId])

  async function onSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setPending(true)
    setMessage('')
    const result = await adminJson<{ version?: number }>(`/api/admin/projects/${projectId}`, {
      method: 'PUT',
      body: JSON.stringify({ expectedVersion: version, title, summary, status }),
    })
    setPending(false)
    if (result.status === 409) {
      setMessage('다른 수정이 먼저 저장되었다.')
      await load()
      return
    }
    if (result.status !== 200 || !result.body?.version) {
      setMessage('설명을 저장하지 못했다.')
      return
    }
    setVersion(result.body.version)
    setMessage('설명을 저장했다.')
  }

  async function onCreateLog(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setPending(true)
    setMessage('')
    const result = await adminJson(`/api/admin/projects/${projectId}/logs`, {
      method: 'POST',
      body: JSON.stringify({
        slug: logSlug,
        title: logTitle,
        stage,
        displayOrder: order,
        commitUrl,
      }),
    })
    setPending(false)
    if (result.status === 422) {
      setMessage('단계, 순서, 커밋 주소를 다시 확인해 달라.')
      return
    }
    if (result.status === 409) {
      setMessage('같은 슬러그의 과정이 이미 있다.')
      return
    }
    if (result.status !== 201) {
      setMessage('과정을 연결하지 못했다.')
      return
    }
    setLogSlug('')
    setLogTitle('')
    setStage('')
    setCommitUrl('')
    setMessage('과정을 연결했다.')
    await load()
  }

  function move(index: number, delta: number) {
    const next = index + delta
    if (next < 0 || next >= logs.length) return
    const copy = logs.slice()
    const [item] = copy.splice(index, 1)
    copy.splice(next, 0, item)
    setLogs(copy)
  }

  async function onOrder() {
    setPending(true)
    setMessage('')
    const result = await adminJson(`/api/admin/projects/${projectId}/logs/order`, {
      method: 'PUT',
      body: JSON.stringify({ entryIds: logs.map((item) => item.id) }),
    })
    setPending(false)
    if (result.status !== 200) {
      setMessage('순서를 저장하지 못했다.')
      await load()
      return
    }
    setMessage('과정 순서를 저장했다.')
    await load()
  }

  if (!csrf) return signedOut(project?.title || '프로젝트 관리')

  return (
    <AdminFrame title={title || '프로젝트 관리'} label="설명 편집 · 과정 연결 · 과정 순서 변경" current="projects">
      {message ? <p className="dg-item mb-4 px-4 py-3" role="status">{message}</p> : null}
      {!project && !message ? <p>프로젝트를 불러오는 중이다.</p> : null}
      {project ? (
        <>
          <form className="dg-panel mb-4 grid gap-3 p-5" onSubmit={onSave}>
            <h2 className="mb-0 text-xl">설명 편집</h2>
            <p className="mb-0 text-sm text-quiet">주 프로젝트 · {project.title}</p>
            <label htmlFor="edit-title">프로젝트 이름</label>
            <input className="dg-input" id="edit-title" value={title} onChange={(event) => setTitle(event.target.value)} />
            <label htmlFor="edit-summary">한 줄 설명</label>
            <textarea className="dg-input" id="edit-summary" value={summary} onChange={(event) => setSummary(event.target.value)} />
            <label htmlFor="edit-status">상태</label>
            <select className="dg-input" id="edit-status" value={status} onChange={(event) => setStatus(event.target.value)}>
              {statuses.map((item) => (
                <option key={item} value={item}>{item}</option>
              ))}
            </select>
            <button className="dg-btn" type="submit" disabled={pending}>설명 저장</button>
          </form>
          <form className="dg-panel mb-4 grid gap-3 p-5" onSubmit={onCreateLog}>
            <h2 className="mb-0 text-xl">과정 연결</h2>
            <label htmlFor="log-title">과정 제목</label>
            <input className="dg-input" id="log-title" value={logTitle} onChange={(event) => setLogTitle(event.target.value)} />
            <label htmlFor="log-slug">슬러그</label>
            <input className="dg-input" id="log-slug" value={logSlug} onChange={(event) => setLogSlug(event.target.value)} />
            <label htmlFor="log-stage">단계</label>
            <input className="dg-input" id="log-stage" value={stage} onChange={(event) => setStage(event.target.value)} />
            <label htmlFor="log-order">순서</label>
            <input className="dg-input" id="log-order" type="number" min={0} max={10000} value={order} onChange={(event) => setOrder(Number(event.target.value))} />
            <label htmlFor="log-commit">관련 커밋</label>
            <input className="dg-input" id="log-commit" value={commitUrl} onChange={(event) => setCommitUrl(event.target.value)} placeholder="https://" />
            <button className="dg-btn" type="submit" disabled={pending}>과정 연결</button>
          </form>
          <section className="dg-panel grid gap-3 p-5" id="logs">
            <h2 className="mb-0 text-xl">과정 순서 변경</h2>
            {logs.length === 0 ? <p className="mb-0">과정 기록이 없다.</p> : null}
            <ol className="m-0 grid list-none gap-2 p-0">
              {logs.map((item, index) => (
                <li key={item.id} className="dg-item flex flex-wrap items-center justify-between gap-2 px-3 py-2">
                  <span>{String(index + 1).padStart(2, '0')} · {item.title} · {item.stage}</span>
                  <span className="flex gap-2">
                    <button className="dg-btn-ghost" type="button" onClick={() => move(index, -1)}>위로</button>
                    <button className="dg-btn-ghost" type="button" onClick={() => move(index, 1)}>아래로</button>
                  </span>
                </li>
              ))}
            </ol>
            {logs.length > 1 ? (
              <button className="dg-btn" type="button" onClick={() => void onOrder()} disabled={pending}>순서 저장</button>
            ) : null}
          </section>
        </>
      ) : null}
    </AdminFrame>
  )
}
