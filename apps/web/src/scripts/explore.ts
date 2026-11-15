/**
 * 기능: 탐색 화면의 검색 상태.
 * 목적: 로딩, 결과, 결과 없음, 429, 503을 나누고 늦게 도착한 이전 응답은 버린다.
 * 연관: `apps/web/src/site/pages.tsx`의 `#explore-form`, `GET /api/search`.
 */

export type SearchView = 'idle' | 'loading' | 'results' | 'empty' | 'limited' | 'unavailable' | 'invalid'

/**
 * 기능: HTTP 상태를 화면 상태로 바꾼다.
 * 목적: 결과 없음과 서버 오류를 같은 문장으로 보이지 않게 한다.
 * 연관: `setupExplore`.
 */
export function viewFor(status: number, count: number): SearchView {
  if (status === 429) return 'limited'
  if (status === 422) return 'invalid'
  if (status !== 200) return 'unavailable'
  return count === 0 ? 'empty' : 'results'
}

/**
 * 기능: 요청 순서가 아직 현재인지 본다.
 * 목적: 먼저 보낸 검색이 나중에 도착해도 화면을 덮지 않게 한다.
 * 연관: `setupExplore`의 generation.
 */
export function acceptResponse(sent: number, current: number): boolean {
  return sent === current
}

const copy: Record<SearchView, string> = {
  idle: '검색어를 입력하세요',
  loading: '검색하고 있어요',
  results: '검색 결과',
  empty: '검색 결과가 없어요',
  limited: '요청이 너무 많아요. 잠시 후 다시 검색해 주세요.',
  unavailable: '검색이 잠시 멈췄어요',
  invalid: '검색어를 확인해 주세요',
}

/**
 * 기능: 탐색 폼에 검색을 붙인다.
 * 목적: 빈 검색어는 서버를 부르지 않고, 응답은 순서대로만 화면에 반영한다.
 * 연관: `/api/search`. 질문 요청은 보내지 않는다.
 */
export function setupExplore(form: HTMLFormElement | null) {
  if (!form) return
  const status = document.querySelector('#explore-status')
  const results = document.querySelector('#explore-results')
  if (!status || !results) return
  let generation = 0
  form.addEventListener('submit', (event) => {
    event.preventDefault()
    const data = new FormData(form)
    const query = String(data.get('q') ?? '').trim()
    if (!query) {
      generation += 1
      status.textContent = copy.idle
      results.replaceChildren()
      return
    }
    const sent = ++generation
    status.textContent = copy.loading
    const type = String(data.get('type') ?? 'all')
    const project = String(data.get('project') ?? '').trim()
    const address = `/api/search?q=${encodeURIComponent(query)}&type=${encodeURIComponent(type)}&project=${encodeURIComponent(project)}`
    void fetch(address)
      .then(async (response) => {
        if (!acceptResponse(sent, generation)) return
        const body = response.ok ? await response.json() : null
        const count = Array.isArray(body?.results) ? body.results.length : 0
        const view = viewFor(response.status, count)
        status.textContent = copy[view]
        results.replaceChildren()
        if (view !== 'results') return
        for (const item of body.results) {
          const line = document.createElement('li')
          line.className = 'dg-item px-4 py-3'
          const link = document.createElement('a')
          link.className = 'text-mark'
          link.href = String(item.url ?? '/explore')
          link.textContent = String(item.title ?? '결과')
          const snippet = document.createElement('p')
          snippet.textContent = String(item.snippet ?? '')
          line.append(link, snippet)
          results.append(line)
        }
      })
      .catch(() => {
        if (!acceptResponse(sent, generation)) return
        status.textContent = copy.unavailable
        results.replaceChildren()
      })
  })
}
