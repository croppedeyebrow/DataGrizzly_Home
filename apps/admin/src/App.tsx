/**
 * 개념: 관리자 화면의 경로 선택.
 * 목적: 로그인, 글 목록, 블록 편집을 각각의 주소에 연결한다.
 * 사용처: apps/admin/src/main.tsx.
 */
import Editor from './editor/Editor'
import EntryList from './entries/EntryList'
import Login from './Login'

type AdminRoute = { name: 'login' } | { name: 'entries' } | { name: 'edit'; id: string }

/**
 * 개념: 주소에서 관리 화면을 고른다.
 * 목적: 라우터 패키지 없이 /admin 아래 경로를 구분한다.
 * 사용처: App.
 */
function parseRoute(pathname: string): AdminRoute {
  const path = pathname.replace(/\/+$/, '')
  if (path === '/admin/entries') return { name: 'entries' }
  const edit = /^\/admin\/entries\/([0-9a-f-]{36})\/edit$/.exec(path)
  if (edit) return { name: 'edit', id: edit[1] }
  return { name: 'login' }
}

/**
 * 개념: 현재 주소의 관리 화면.
 * 목적: 로그인 뒤에 목록과 편집기를 연다.
 * 사용처: main.tsx.
 */
export default function App() {
  const route = parseRoute(window.location.pathname)
  if (route.name === 'entries') return <EntryList />
  if (route.name === 'edit') return <Editor entryId={route.id} />
  return <Login />
}
