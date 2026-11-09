/**
 * 개념: 이 탭의 소유자 세션 보조 저장.
 * 목적: CSRF와 미저장 초안을 새로고침 동안만 유지하고, 로그아웃하면 둘 다 지운다.
 * 사용처: 로그인, 목록, 편집 화면.
 */

const CSRF_KEY = 'dg-csrf'
const DRAFT_PREFIX = 'dg-draft:'

/**
 * 개념: 저장된 CSRF를 읽는다.
 * 목적: 새로고침 뒤에도 같은 탭에서 관리 API를 호출한다.
 * 사용처: 목록과 편집 화면의 첫 요청.
 */
export function readCsrf(): string {
  return sessionStorage.getItem(CSRF_KEY) ?? ''
}

/**
 * 개념: 로그인 응답의 CSRF를 이 탭에 둔다.
 * 목적: 쿠키만으로 저장 요청을 보내지 않는다.
 * 사용처: 로그인 성공.
 */
export function writeCsrf(token: string): void {
  sessionStorage.setItem(CSRF_KEY, token)
}

/**
 * 개념: 글 하나의 미저장 초안 키.
 * 목적: 서버 저장본과 브라우저 임시본의 저장소를 나눈다.
 * 사용처: 편집 화면.
 */
export function draftKey(entryId: string): string {
  return `${DRAFT_PREFIX}${entryId}`
}

/**
 * 개념: 소유자 보조 저장을 비운다.
 * 목적: 로그아웃 뒤에 다른 글의 미저장 내용이 탭에 남지 않게 한다.
 * 사용처: 로그아웃 성공.
 */
export function clearOwnerSession(): void {
  const keys: string[] = []
  for (let index = 0; index < sessionStorage.length; index += 1) {
    const key = sessionStorage.key(index)
    if (key === CSRF_KEY || (key && key.startsWith(DRAFT_PREFIX))) keys.push(key)
  }
  keys.forEach((key) => sessionStorage.removeItem(key))
}
