/**
 * 개념: 관리 API 호출.
 * 목적: 쿠키와 CSRF를 함께 보내고, 세션이 끝나면 로그인으로 돌려보낸다.
 * 사용처: 프로젝트, 분류, 미디어 화면.
 */
import { clearOwnerSession, readCsrf } from './session'

export async function adminJson<T>(path: string, init?: RequestInit): Promise<{ status: number; body: T | null }> {
  const headers = new Headers(init?.headers)
  headers.set('x-csrf-token', readCsrf())
  if (init?.body) headers.set('content-type', 'application/json')
  try {
    const response = await fetch(path, { ...init, credentials: 'include', headers })
    if (response.status === 401) {
      clearOwnerSession()
      window.location.assign('/admin/login')
      return { status: 401, body: null }
    }
    const text = await response.text()
    if (!text) return { status: response.status, body: null }
    return { status: response.status, body: JSON.parse(text) as T }
  } catch {
    return { status: 0, body: null }
  }
}
