/**
 * 기능: 로그인부터 철회까지의 로컬 확인.
 * 실행: OWNER_PASSWORD를 환경에만 두고 `node infra/ops/journey.mjs`
 * 목적: 작성, 첨부, 미리보기, 발행, 검색, 철회가 한 글에서 이어지는지 본다.
 * 연관: `/api/admin/session`, `/api/admin/entries`, `/api/admin/media`, `/api/search`, `services/api/src/snapshot.rs`.
 * 비밀번호는 출력하지 않는다. 이 주소는 공개 도메인이 아니다.
 */
const base = process.env.JOURNEY_BASE ?? 'http://127.0.0.1:8080'
const password = process.env.OWNER_PASSWORD ?? ''
if (!password) {
  console.log(JSON.stringify({ ok: false, code: 'owner_password_missing' }))
  process.exit(1)
}

const token = 'm6opsjourney'
const png = Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), Buffer.from('png-test')]).toString('base64')
let cookie = ''
let csrf = ''

const report = { ok: false, base, steps: [] }

try {
  const loggedIn = await send('POST', '/api/admin/session', { password })
  csrf = loggedIn.json?.csrfToken ?? ''
  report.steps.push(step('login', loggedIn.status, Boolean(csrf)))
  const created = await send('POST', '/api/admin/entries', { slug: 'm6-ops-check', title: '운영 확인 메모', kind: 'note' })
  const entryId = created.json?.id
  report.steps.push(step('write', created.status, Boolean(entryId)))
  const saved = await send('PUT', `/api/admin/entries/${entryId}`, {
    expectedVersion: 1,
    title: '운영 확인 메모',
    summary: '',
    blocks: [{ id: '11111111-1111-4111-8111-111111111111', type: 'paragraph', text: `${token} 확인 문장` }],
    metadata: { type: 'note' },
  })
  report.steps.push(step('save', saved.status, Boolean(saved.json?.draftRevisionId)))
  const uploaded = await send('POST', '/api/admin/media', { filename: 'check.png', contentBase64: png })
  const mediaId = uploaded.json?.id
  report.steps.push(step('attach-upload', uploaded.status, Boolean(mediaId)))
  const labeled = await send('PUT', `/api/admin/media/${mediaId}`, { alt: '운영 확인 첨부', caption: '' })
  report.steps.push(step('attach-text', labeled.status, labeled.json?.ok === true))
  const used = await send('POST', `/api/admin/media/${mediaId}/usages`, { entryId })
  report.steps.push(step('attach-usage', used.status, used.status === 200 || used.status === 201))
  const preview = await send('GET', `/api/admin/entries/${entryId}/preview`)
  const revisionId = preview.json?.revisionId
  report.steps.push(step('preview', preview.status, revisionId === saved.json?.draftRevisionId))
  const accepted = await send('POST', `/api/admin/entries/${entryId}/publications`, {
    schemaVersion: 1,
    revisionId,
    idempotencyKey: 'm6.ops.journey-1',
  })
  const jobId = accepted.json?.jobId
  report.steps.push(step('publish-accept', accepted.status, Boolean(jobId)))
  const polled = await send('POST', '/api/admin/publication-jobs/poll', {})
  report.steps.push(step('publish-poll', polled.status, polled.json?.claimedJobId === jobId))
  const advanced = await send('POST', `/api/admin/publication-jobs/${jobId}/advance`, { fence: polled.json?.fence })
  report.steps.push(step('publish-advance', advanced.status, advanced.status === 200))
  const released = await send('POST', `/api/admin/publication-jobs/${jobId}/release`)
  report.steps.push(step('publish-release', released.status, released.json?.succeeded === true))
  report.release = { succeeded: released.json?.succeeded === true, code: released.json?.code ?? null }
  const found = await send('GET', `/api/search?q=${token}&type=note`)
  const hit = Array.isArray(found.json?.results) && found.json.results.some((item) => String(item.url ?? '').includes('m6-ops-check'))
  report.steps.push(step('search', found.status, hit))
  const current = await send('GET', `/api/admin/entries/${entryId}`)
  const withdrawn = await send('POST', `/api/admin/entries/${entryId}/unpublish`, {
    schemaVersion: 1,
    expectedVersion: current.json?.version,
  })
  report.steps.push(step('withdraw', withdrawn.status, withdrawn.status === 202 || withdrawn.status === 200))
  const after = await send('GET', `/api/search?q=${token}&type=note`)
  const hidden = after.status === 200 && Array.isArray(after.json?.results) && !after.json.results.some((item) => String(item.url ?? '').includes('m6-ops-check'))
  report.steps.push(step('search-after-withdraw', after.status, hidden))
  report.ok = report.steps.every((item) => item.pass)
} catch (error) {
  report.code = 'journey_failed'
  report.message = error instanceof Error ? error.message : 'journey_failed'
}

console.log(JSON.stringify(report))
process.exit(report.ok ? 0 : 1)

function step(name, status, pass) {
  return { name, status, pass }
}

async function send(method, path, body) {
  const headers = { accept: 'application/json' }
  if (cookie) headers.cookie = cookie
  if (csrf) headers['x-csrf-token'] = csrf
  let payload
  if (body !== undefined && method !== 'GET') {
    headers['content-type'] = 'application/json'
    payload = JSON.stringify(body)
  }
  const response = await fetch(`${base}${path}`, { method, headers, body: payload })
  const setCookie = response.headers.get('set-cookie')
  if (setCookie) cookie = setCookie.split(';')[0]
  const text = await response.text()
  let json = null
  try {
    json = text ? JSON.parse(text) : null
  } catch {
    json = null
  }
  return { status: response.status, json }
}
