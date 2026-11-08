/**
 * 개념: 소유자 로그인 화면.
 * 목적: 비밀번호를 API로 보내고, 돌아온 CSRF로만 로그아웃한다.
 * 사용처: apps/admin/src/main.tsx. 경로는 /admin/ 와 /admin/login.
 */
import { useState, type FormEvent } from 'react'

const FAILURE = '로그인에 실패했다. 다시 시도해 달라.'

export default function App() {
  const [password, setPassword] = useState('')
  const [csrfToken, setCsrfToken] = useState('')
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
        setCsrfToken('')
        setMessage(FAILURE)
        return
      }
      const body = (await response.json()) as { csrfToken?: string }
      if (!body.csrfToken) {
        setMessage(FAILURE)
        return
      }
      setCsrfToken(body.csrfToken)
      setMessage('')
    } catch {
      setCsrfToken('')
      setMessage('서버에 연결하지 못했다. 다시 시도해 달라.')
    } finally {
      setPending(false)
    }
  }

  async function onLogout() {
    setPending(true)
    try {
      const response = await fetch('/api/admin/session', {
        method: 'DELETE',
        credentials: 'include',
        headers: { 'x-csrf-token': csrfToken },
      })
      if (!response.ok) {
        setMessage('로그아웃하지 못했다. 다시 시도해 달라.')
        return
      }
      setCsrfToken('')
      setMessage('')
    } catch {
      setMessage('서버에 연결하지 못했다. 다시 시도해 달라.')
    } finally {
      setPending(false)
    }
  }

  return (
    <main>
      <h1>소유자 로그인</h1>
      {csrfToken ? (
        <>
          <p>로그인되었다.</p>
          <button type="button" onClick={onLogout} disabled={pending}>
            로그아웃
          </button>
        </>
      ) : (
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
      )}
      {message ? <p role="alert">{message}</p> : null}
      <p>
        <a href="http://127.0.0.1:4321/">공개 홈페이지로 돌아가기</a>
      </p>
    </main>
  )
}
