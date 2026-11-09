/**
 * 개념: 소유자 로그인 폼.
 * 목적: 비밀번호를 API로 보내고 CSRF를 이 탭에만 남긴다.
 * 사용처: /admin/ 와 /admin/login.
 */
import { useState, type FormEvent } from 'react'
import { writeCsrf } from './session'

const FAILURE = '이유를 확인하고 다시 시도하세요.'

/**
 * 개념: 로그인 화면.
 * 목적: 실패를 성공 화면으로 넘기지 않는다.
 * 사용처: App의 로그인 경로.
 */
export default function Login() {
  const [password, setPassword] = useState('')
  const [message, setMessage] = useState('')
  const [pending, setPending] = useState(false)

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!password) {
      setMessage('이유를 확인하고 다시 시도하세요.')
      return
    }
    setPending(true)
    setMessage('')
    const entered = password
    setPassword('')
    try {
      const response = await fetch('/api/admin/session', {
        method: 'POST',
        credentials: 'include',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ password: entered }),
      })
      if (!response.ok) {
        setMessage(FAILURE)
        return
      }
      const body = (await response.json()) as { csrfToken?: string }
      if (!body.csrfToken) {
        setMessage(FAILURE)
        return
      }
      writeCsrf(body.csrfToken)
      window.location.assign('/admin/entries')
    } catch {
      setMessage('서버에 연결하지 못했다. 다시 시도해 달라.')
    } finally {
      setPending(false)
    }
  }

  return (
    <main className="grid min-h-screen place-items-center px-4 py-10">
      <section className="dg-hero w-full max-w-lg p-8">
        <p className="mb-6 flex items-center gap-3">
          <span className="dg-icon-tile size-9 text-sm" aria-hidden="true">D</span>
          DataGrizzly 관리자
        </p>
        <h1 className="mb-2 text-3xl">관리자 로그인</h1>
        <p className="mb-6 text-quiet">소유자 계정만 접근할 수 있습니다.</p>
        <form onSubmit={onSubmit}>
          <label htmlFor="owner-password">비밀번호</label>
          <input
            className="dg-input mb-4 w-full"
            id="owner-password"
            type="password"
            autoComplete="current-password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
          />
          <button className="dg-btn w-full" type="submit" disabled={pending}>
            설정한 인증 방식으로 로그인
          </button>
        </form>
        {message ? (
          <p className="dg-item mt-4 px-4 py-3" role="alert">
            {message}
          </p>
        ) : null}
        <p className="mt-6 mb-0 text-quiet">인증 방식은 M2-01 명세에서 확정합니다.</p>
      </section>
    </main>
  )
}
