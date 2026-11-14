/**
 * 개념: 발행 작업 화면.
 * 목적: 접수한 작업의 상태와 실패를 보여 주고, 202 접수를 공개 완료로 적지 않는다.
 * 사용처: /admin/publications.
 */
import { useEffect, useState } from 'react'
import { AdminFrame } from '../AdminFrame'
import { adminJson } from '../api'
import { readCsrf } from '../session'

type JobRow = {
  id: string
  jobNumber: number
  revisionId: string
  revisionVersion: number
  state: string
  failureCode: string | null
  releaseId: string | null
  retryable: boolean
}

type ScheduleRow = {
  id: string
  displayAt: string
  runAt: string
  version: number
  state: string
  cancelable: boolean
}

type WithdrawalRow = {
  id: string
  entryId: string
  state: string
  retryable: boolean
  memo: string
}

const STEPS = ['대기', '빌드', '배포', '확인']

function stepIndex(state: string): number {
  if (state === 'building') return 1
  if (state === 'deploying') return 2
  if (state === 'succeeded') return 3
  return 0
}

function stateLabel(state: string): string {
  if (state === 'queued') return '대기'
  if (state === 'building') return '빌드'
  if (state === 'deploying') return '배포'
  if (state === 'succeeded') return '완료'
  if (state === 'failed') return '실패'
  if (state === 'canceled') return '취소'
  return state
}

/**
 * 개념: 실패 코드의 화면 문장.
 * 목적: 내부 경로나 비밀 없이 운영자가 다음 행동을 알게 한다.
 * 사용처: 작업 표의 메모.
 */
function memo(job: JobRow): string {
  if (job.failureCode === 'retry_exhausted') return '자동 재시도 횟수를 모두 썼다'
  if (job.failureCode === 'draft_included') return '초안 표시가 있어 배포하지 않았다'
  if (job.failureCode === 'missing_asset' || job.failureCode === 'missing_sitemap') return '후보 파일이 없어 배포하지 않았다'
  if (job.failureCode === 'marker_mismatch') return '공개 표시가 달라 성공으로 기록하지 않았다'
  if (job.failureCode === 'db_confirm_failed') return '포인터는 바뀌었고 DB 성공 기록은 아직이다'
  if (job.failureCode === 'build_failed' || job.failureCode === 'candidate_write_failed' || job.failureCode === 'pointer_failed') return '후보를 만들지 못해 기존 공개본을 유지했다'
  if (job.failureCode === 'stale_generation') return '더 나중에 확인된 세대가 있어 이 작업은 덮지 않았다'
  if (job.failureCode === 'withdrawn_entry') return '철회된 글이라 공개 포인터를 바꾸지 않았다'
  if (job.state === 'canceled') return '접수 취소'
  if (job.state === 'succeeded' && job.releaseId) return `공개 세대 ${job.releaseId}`
  if (job.state === 'succeeded') return '공개 표시를 확인했다'
  if (job.state === 'deploying') return '공개 확인 전'
  return '진행 중'
}

function clock(date: Date): string {
  const pad = (value: number) => String(value).padStart(2, '0')
  return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}

/**
 * 개념: 작업 목록과 진행 단계.
 * 목적: 고른 작업의 실제 상태만 단계에 표시하고, 없는 작업은 예시 번호로 채우지 않는다.
 * 사용처: App.
 */
export default function Publications() {
  const csrf = readCsrf()
  const [jobs, setJobs] = useState<JobRow[] | null>(null)
  const [withdrawals, setWithdrawals] = useState<WithdrawalRow[] | null>(null)
  const [schedules, setSchedules] = useState<ScheduleRow[] | null>(null)
  const [selectedId, setSelectedId] = useState('')
  const [message, setMessage] = useState('')
  const [refreshed, setRefreshed] = useState('')
  const [pending, setPending] = useState(false)

  async function load() {
    const result = await adminJson<{ jobs?: JobRow[] }>('/api/admin/publication-jobs')
    if (result.status === 401) return
    if (result.status !== 200 || !result.body?.jobs) {
      setMessage(result.status === 0 ? '서버에 연결하지 못했다.' : '발행 작업을 불러오지 못했다.')
      return
    }
    setJobs(result.body.jobs)
    const withdrawn = await adminJson<{ withdrawals?: WithdrawalRow[] }>('/api/admin/withdrawals')
    setWithdrawals(withdrawn.status === 200 ? withdrawn.body?.withdrawals ?? [] : null)
    const scheduled = await adminJson<{ schedules?: ScheduleRow[] }>('/api/admin/schedules')
    setSchedules(scheduled.status === 200 ? scheduled.body?.schedules ?? [] : null)
    setMessage('')
    setRefreshed(clock(new Date()))
    setSelectedId((current) => current || result.body?.jobs?.[0]?.id || '')
  }

  useEffect(() => {
    if (!csrf) return
    void load()
  }, [csrf])

  async function onRetry(id: string) {
    setPending(true)
    const result = await adminJson(`/api/admin/publication-jobs/${id}/retry`, { method: 'POST' })
    setPending(false)
    if (result.status === 409) {
      setMessage('이 작업은 다시 대기열에 넣을 수 없다.')
      await load()
      return
    }
    if (result.status !== 200) {
      setMessage('재시도하지 못했다.')
      return
    }
    setMessage('대기열로 되돌렸다. 공개본은 바뀌지 않았다.')
    await load()
  }

  // 배포 대기 작업을 확인하고, 릴리스 표시가 맞을 때만 성공 응답을 받는다.
  async function onConfirm(id: string) {
    setPending(true)
    const result = await adminJson<{ succeeded?: boolean; failureCode?: string | null }>(`/api/admin/publication-jobs/${id}/release`, { method: 'POST' })
    setPending(false)
    if (result.body?.succeeded) {
      setMessage('공개 표시를 확인한 뒤 성공으로 기록했다.')
    } else if (result.body?.failureCode) {
      setMessage('확인 전에는 성공으로 기록하지 않았다.')
    } else if (result.status === 409) {
      setMessage('배포 대기 작업만 확인할 수 있다.')
    } else {
      setMessage('후보를 확인하지 못했다.')
    }
    await load()
  }

  async function onRetryWithdrawal(id: string) {
    setPending(true)
    const result = await adminJson<{ state?: string }>(`/api/admin/withdrawals/${id}/retry`, { method: 'POST' })
    setPending(false)
    await load()
    if (result.body?.state === 'blocked') {
      setMessage('로컬 회수 표시를 다시 썼다. 외부 캐시 삭제는 보장하지 않는다.')
    } else if (result.status === 409) {
      setMessage('회수에 실패한 철회만 다시 시도한다.')
    } else {
      setMessage('회수 표시를 쓰지 못했다. 공개 API 차단은 유지된다.')
    }
  }

  async function onRunDue() {
    setPending(true)
    const result = await adminJson<{ enqueued?: boolean }>('/api/admin/schedules/run', { method: 'POST' })
    setPending(false)
    await load()
    if (result.body?.enqueued) {
      setMessage('놓친 예약을 대기열에 넣었다. 공개본은 바뀌지 않았다.')
    } else if (result.status === 200) {
      setMessage('실행할 예약이 없다.')
    } else {
      setMessage('예약을 확인하지 못했다.')
    }
  }

  async function onCancelSchedule(id: string, version: number) {
    setPending(true)
    const result = await adminJson<{ state?: string }>(`/api/admin/schedules/${id}/cancel`, {
      method: 'POST',
      body: JSON.stringify({ schemaVersion: 1, expectedVersion: version }),
    })
    setPending(false)
    await load()
    if (result.body?.state === 'canceled') {
      setMessage('예약을 취소했다. 공개본은 바뀌지 않았다.')
    } else {
      setMessage('실행 전 예약만 취소할 수 있다.')
    }
  }

  async function onCancel(id: string) {
    setPending(true)
    const result = await adminJson(`/api/admin/publication-jobs/${id}/cancel`, { method: 'POST' })
    setPending(false)
    if (result.status === 409) {
      setMessage('대기 중인 접수만 취소할 수 있다.')
      await load()
      return
    }
    if (result.status !== 200) {
      setMessage('취소하지 못했다.')
      return
    }
    setMessage('접수를 취소했다. 공개본은 바뀌지 않았다.')
    await load()
  }

  const selected = jobs?.find((job) => job.id === selectedId) ?? jobs?.[0]
  const activeStep = selected ? stepIndex(selected.state) : -1

  if (!csrf) {
    return (
      <AdminFrame title="발행 작업" label="작업 ID · 대상 수정본 · 공개 세대" current="publications">
        <p className="dg-item px-4 py-3" role="alert">로그인이 필요하다.</p>
        <p>
          <a className="dg-btn" href="/admin/login">소유자 로그인</a>
        </p>
      </AdminFrame>
    )
  }

  return (
    <AdminFrame
      title="발행 작업"
      label="작업 ID · 대상 수정본 · 공개 세대"
      current="publications"
      action={
        <button className="dg-btn-ghost" type="button" onClick={() => void load()}>
          새로고침
        </button>
      }
    >
      <p className="dg-item mb-4 px-4 py-3">접수는 공개 완료가 아닙니다. 확인 전에는 공개본이 바뀌지 않습니다.</p>
      <section className="dg-panel mb-4 p-4">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          <h2 className="mb-0 text-lg">예약</h2>
          <button className="dg-btn-ghost" type="button" disabled={pending} onClick={() => void onRunDue()}>
            놓친 예약 실행
          </button>
        </div>
        <p className="mb-3 text-sm">놓친 예약은 서버가 다시 확인하는 즉시 대기열에 넣는다. 같은 예약은 한 번만 작업을 만든다. 예약 실행은 공개 완료가 아니다.</p>
        {schedules && schedules.length === 0 ? <p className="mb-0">예약이 없다.</p> : null}
        {schedules && schedules.length > 0 ? (
          <ul className="m-0 grid list-none gap-2 p-0">
            {schedules.map((item) => (
              <li key={item.id} className="flex flex-wrap items-center justify-between gap-2">
                <span>서울 {item.displayAt} · UTC {item.runAt} · {item.state}</span>
                {item.cancelable ? (
                  <button className="dg-btn-ghost" type="button" disabled={pending} onClick={() => void onCancelSchedule(item.id, item.version)}>
                    예약 취소
                  </button>
                ) : null}
              </li>
            ))}
          </ul>
        ) : null}
      </section>
      {message ? <p className="dg-item mb-4 px-4 py-3" role="status">{message}</p> : null}
      <section className="dg-panel mb-4 p-4">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          <h2 className="mb-0 text-lg">진행 중 · {selected ? `job-[${selected.jobNumber}]` : '작업 없음'}</h2>
          <p className="mb-0 text-sm text-quiet">{refreshed ? `마지막 갱신 ${refreshed}` : '아직 갱신하지 않았다'}</p>
        </div>
        <ol className="m-0 grid list-none gap-2 p-0 min-[40rem]:grid-cols-4">
          {STEPS.map((label, index) => (
            <li key={label} className={index === activeStep ? 'dg-chip' : 'text-quiet'}>
              {index + 1} {label}
            </li>
          ))}
        </ol>
        {selected && (selected.state === 'failed' || selected.state === 'canceled') ? (
          <p className="mb-0 mt-3 text-sm">확인 단계에 도달하지 않았다.</p>
        ) : null}
      </section>
      {withdrawals && withdrawals.length > 0 ? (
        <section className="dg-panel mb-4 p-4">
          <h2 className="mb-3 text-lg">철회</h2>
          <ul className="m-0 grid list-none gap-2 p-0">
            {withdrawals.map((item) => (
              <li key={item.id} className="flex flex-wrap items-center justify-between gap-2">
                <span>{item.memo}</span>
                {item.retryable ? (
                  <button className="dg-btn-ghost" type="button" disabled={pending} onClick={() => void onRetryWithdrawal(item.id)}>
                    회수 재시도
                  </button>
                ) : null}
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      {withdrawals && withdrawals.length === 0 ? <p className="dg-item mb-4 px-4 py-3">철회된 글이 없다.</p> : null}
      {jobs === null && !message ? <p>발행 작업을 불러오는 중이다.</p> : null}
      {jobs && jobs.length === 0 ? <p className="dg-item px-4 py-3">발행 작업이 없다.</p> : null}
      {jobs && jobs.length > 0 ? (
        <div className="dg-panel overflow-x-auto p-4">
          <table className="w-full border-collapse text-left">
            <thead>
              <tr>
                <th className="px-2 py-2">작업 ID</th>
                <th className="px-2 py-2">대상 수정본</th>
                <th className="px-2 py-2">상태</th>
                <th className="px-2 py-2">메모</th>
                <th className="px-2 py-2">작업</th>
              </tr>
            </thead>
            <tbody>
              {jobs.map((job) => (
                <tr key={job.id}>
                  <td className="px-2 py-2">
                    <button className="dg-btn-ghost" type="button" onClick={() => setSelectedId(job.id)}>
                      job-[{job.jobNumber}]
                    </button>
                  </td>
                  <td className="px-2 py-2">r-[{String(job.revisionVersion).padStart(2, '0')}]</td>
                  <td className="px-2 py-2"><span className="dg-chip">{stateLabel(job.state)}</span></td>
                  <td className="px-2 py-2">{memo(job)}</td>
                  <td className="px-2 py-2">
                    {job.state === 'deploying' ? (
                      <button className="dg-btn-ghost" type="button" disabled={pending} onClick={() => void onConfirm(job.id)}>
                        표시 확인
                      </button>
                    ) : job.retryable ? (
                      <button className="dg-btn-ghost" type="button" disabled={pending} onClick={() => void onRetry(job.id)}>
                        재시도
                      </button>
                    ) : job.state === 'queued' ? (
                      <button className="dg-btn-ghost" type="button" disabled={pending} onClick={() => void onCancel(job.id)}>
                        접수 취소
                      </button>
                    ) : (
                      <button className="dg-btn-ghost" type="button" onClick={() => setSelectedId(job.id)}>
                        상세 보기
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
    </AdminFrame>
  )
}
