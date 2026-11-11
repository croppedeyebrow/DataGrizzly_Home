/**
 * 개념: 관리자 화면의 경로 선택.
 * 목적: 로그인, 글 목록, 블록 편집, 초안 미리보기, 수정 이력을 각각의 주소에 연결한다.
 * 사용처: apps/admin/src/main.tsx.
 */
import Editor from './editor/Editor'
import EntryList from './entries/EntryList'
import Login from './Login'
import MediaLibrary from './media/MediaLibrary'
import Preview from './preview/Preview'
import { ProjectEditor, ProjectList } from './projects/Projects'
import Revisions from './revisions/Revisions'
import Taxonomies from './taxonomies/Taxonomies'

type AdminRoute =
  | { name: 'login' }
  | { name: 'entries' }
  | { name: 'edit'; id: string }
  | { name: 'preview'; id: string }
  | { name: 'revisions'; id: string }
  | { name: 'projects' }
  | { name: 'project'; id: string }
  | { name: 'taxonomies' }
  | { name: 'media' }

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
  // 편집 주소와 끝이 다르므로 미리보기와 이력이 편집 화면으로 겹치지 않는다.
  const preview = /^\/admin\/entries\/([0-9a-f-]{36})\/preview$/.exec(path)
  if (preview) return { name: 'preview', id: preview[1] }
  const revisions = /^\/admin\/entries\/([0-9a-f-]{36})\/revisions$/.exec(path)
  if (revisions) return { name: 'revisions', id: revisions[1] }
  if (path === '/admin/projects') return { name: 'projects' }
  const project = /^\/admin\/projects\/([0-9a-f-]{36})$/.exec(path)
  if (project) return { name: 'project', id: project[1] }
  if (path === '/admin/taxonomies') return { name: 'taxonomies' }
  if (path === '/admin/media') return { name: 'media' }
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
  // 미리보기와 이력은 글 목록과 같은 게시물 메뉴에 속한다.
  if (route.name === 'preview') return <Preview entryId={route.id} />
  if (route.name === 'revisions') return <Revisions entryId={route.id} />
  if (route.name === 'projects') return <ProjectList />
  if (route.name === 'project') return <ProjectEditor projectId={route.id} />
  if (route.name === 'taxonomies') return <Taxonomies />
  if (route.name === 'media') return <MediaLibrary />
  return <Login />
}
