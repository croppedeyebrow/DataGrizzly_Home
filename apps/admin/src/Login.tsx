/**
 * 개념: 소유자 로그인 폼.
 * 목적: 비밀번호를 API로 보내고 CSRF를 이 탭에만 남긴다.
 * 사용처: /admin/ 와 /admin/login.
 */
import { useState, type FormEvent } from 'react'
import { writeCsrf } from './session'

const FAILURE = '로그인에 실패했다. 다시 시도해 달라.'

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
      setMessage('비밀번호를 입력해 달라.')
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
    <main>
      <h1>소유자 로그인</h1>
      <form onSubmit={onSubmit}>
        <label htmlFor="owner-password">비밀번호</label>
        <input
          id="owner-password"
          type="password"
          autoComplete="current-password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
        />
        <button type="submit" disabled={pending}>
          로그인
        </button>
      </form>
      {message ? <p role="alert">{message}</p> : null}
      <p>
        <a href="http://127.0.0.1:4321/">공개 홈페이지로 돌아가기</a>
      </p>
    </main>
  )
}
