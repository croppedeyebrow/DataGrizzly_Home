/**
 * 개념: 릴리스 피드 규칙.
 * 목적: 초안을 사이트맵에서 빼고, 슬러그 변경은 301로 남긴다. 검색 서비스는 부르지 않는다.
 * 사용처: 정적 빌드가 같은 규칙을 읽을 때. API의 실행 경로는 Rust seo_sync다.
 */

export function canonicalPath(kind, slug) {
  const section = kind === 'log' ? 'logs' : kind === 'paper' || kind === 'news' ? 'research' : 'notes'
  return `/${section}/${slug}`
}

export function includeInFeed(record) {
  const path = record?.path ?? ''
  return record?.draft !== true && path.startsWith('/') && !path.startsWith('/admin') && !path.includes('data-draft')
}

export function redirectForSlug(previous, next) {
  if (!previous || previous === next) return null
  if (!includeInFeed({ path: previous }) || !includeInFeed({ path: next })) return null
  return { from: previous, to: next, status: 301 }
}

export function indexHook() {
  return { called: false, reason: 'search_unavailable' }
}
