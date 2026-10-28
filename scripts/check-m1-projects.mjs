/**
 * 개념: 프로젝트 페이지가 공개 과정만 고르고, 빌드 HTML이 그 경로를 지키는지 본다.
 * 목적: 이전·다음이 공개 글만 가리키고, 같은 과정 글을 두 주소에 만들지 않게 한다.
 * 사용처: scripts/check.ps1, scripts/check.sh 의 레이아웃 검사 다음 단계.
 */
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { loadPublicProjects, selectPublicProjects } from './public-projects.mjs'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
let failed = false

function fail(message) {
  failed = true
  console.log(`FAIL ${message}`)
}

function entry(id, slug, type, revisionId, extra = {}) {
  return {
    entryId: id,
    slug,
    type,
    example: true,
    publishedRevisionId: revisionId,
    updatedAt: '2026-10-01T00:00:00Z',
    ...extra,
  }
}

function revision(id, entryId, title, metadata, extra = {}) {
  return {
    revisionId: id,
    entryId,
    title,
    example: true,
    createdAt: extra.createdAt ?? '2026-10-01T00:00:00Z',
    metadata,
    blocks: extra.blocks ?? [],
  }
}

function pair(name, entryValue, revisionValue) {
  return {
    entries: { [`entries/${name}.json`]: entryValue },
    revisions: { [`revisions/${name}.json`]: revisionValue },
    item: { role: entryValue.type, entry: `entries/${name}.json`, revision: `revisions/${name}.json` },
  }
}

const missing = loadPublicProjects(path.join(root, 'fixtures', 'content-missing'))
if (!missing.loadError || missing.projects.length !== 0) fail('missing fixture did not stay empty')
else console.log('PASS missing-project-empty')

const other = selectPublicProjects({ dataset: 'other', public: [] }, {}, {})
if (other.projects.length !== 0 || other.loadError) fail('foreign dataset')
else console.log('PASS foreign-dataset')

const projectId = 'p-1'
const project = pair('project', entry(projectId, 'desk', 'project', 'r-p', { updatedAt: '2026-10-02T00:00:00Z' }), revision('r-p', projectId, '책상', { type: 'project' }, {
  blocks: [
    { type: 'heading', anchor: 'problem' },
    { type: 'paragraph', text: '문제 문장' },
    { type: 'heading', anchor: 'status' },
    { type: 'paragraph', text: '예시 상태' },
  ],
}))

function logPair(name, id, slug, order, createdAt, entryId = id) {
  return pair(name, entry(entryId, slug, 'log', `r-${name}`), revision(`r-${name}`, entryId, name, {
    type: 'log',
    projectId,
    stage: 'planning',
    displayOrder: order,
  }, { createdAt }))
}

function select(parts, extraPublic = []) {
  const entries = {}
  const revisions = {}
  const manifest = { dataset: 'public-example', public: [...extraPublic] }
  for (const part of parts) {
    Object.assign(entries, part.entries)
    Object.assign(revisions, part.revisions)
    manifest.public.push(part.item)
  }
  return selectPublicProjects(manifest, entries, revisions)
}

const none = select([project])
if (none.projects.length !== 1 || none.projects[0].logs.length !== 0 || none.logs.length !== 0) fail('zero logs')
else console.log('PASS zero-logs')

const one = select([project, logPair('only', 'only', 'only', 0, '2026-10-03T00:00:00Z')])
if (one.logs.length !== 1 || one.logs[0].previous !== null || one.logs[0].next !== null) fail('one log neighbors')
else console.log('PASS one-log')

const three = select([
  project,
  logPair('read-c', 'c', 'read-c', 2, '2026-10-10T00:00:00Z', 'id-c'),
  logPair('read-a', 'a', 'read-a', 0, '2026-10-20T00:00:00Z', 'id-a'),
  logPair('read-b', 'b', 'read-b', 1, '2026-10-01T00:00:00Z', 'id-b'),
])
const threeSlugs = three.logs.map((log) => log.slug).join(',')
if (threeSlugs !== 'read-a,read-b,read-c') fail(`three log order ${threeSlugs}`)
else if (three.logs[0].next?.slug !== 'read-b' || three.logs[2].previous?.slug !== 'read-b' || three.logs[0].previous || three.logs[2].next) fail('three log neighbors')
else console.log('PASS three-logs')

const hidden = logPair('hidden', 'h', 'hidden', 1, '2026-10-02T00:00:00Z', 'id-h')
const left = logPair('left', 'l', 'left', 0, '2026-10-20T00:00:00Z', 'id-l')
const right = logPair('right', 'r', 'right', 2, '2026-10-03T00:00:00Z', 'id-r')
const publicOnly = selectPublicProjects({
  dataset: 'public-example',
  public: [project.item, left.item, right.item],
}, {
  ...project.entries,
  ...left.entries,
  ...right.entries,
  ...hidden.entries,
}, {
  ...project.revisions,
  ...left.revisions,
  ...right.revisions,
  ...hidden.revisions,
})
if (publicOnly.logs.map((log) => log.slug).join(',') !== 'left,right' || publicOnly.logs[0].next?.slug !== 'right') fail('hidden middle stayed linked')
else console.log('PASS hidden-middle')

const noSlug = select([pair('blank', entry(projectId, '', 'project', 'r-blank'), revision('r-blank', projectId, '없음', { type: 'project' }))])
if (noSlug.projects.length !== 0) fail('blank slug')
else console.log('PASS blank-slug')

const badSlug = select([pair('bad', entry('p-bad', 'Bad Slug', 'project', 'r-bad'), revision('r-bad', 'p-bad', '나쁨', { type: 'project' }))])
if (badSlug.projects.length !== 0) fail('bad slug')
else console.log('PASS bad-slug')

const tied = select([
  project,
  logPair('later', 'z-id', 'later', 1, '2026-10-02T00:00:00Z', 'z-id'),
  logPair('earlier', 'a-id', 'earlier', 1, '2026-10-03T00:00:00Z', 'a-id'),
])
if (tied.logs.map((log) => log.slug).join(',') !== 'earlier,later') fail('tie break')
else console.log('PASS tie-break')

const duplicate = select([
  project,
  logPair('first-copy', 'copy-1', 'same', 0, '2026-10-01T00:00:00Z', 'copy-1'),
  logPair('second-copy', 'copy-2', 'same', 1, '2026-10-02T00:00:00Z', 'copy-2'),
])
if (duplicate.logs.length !== 1 || duplicate.projects[0].logs.length !== 1) fail('duplicate slug')
else console.log('PASS duplicate-slug')

const unpublished = pair('draft', entry('p-draft', 'draft', 'project', 'r-published'), revision('r-other', 'p-draft', '초안', { type: 'project' }))
if (select([unpublished]).projects.length !== 0) fail('unpublished pointer')
else console.log('PASS unpublished-pointer')

const outsider = logPair('outsider', 'out', 'outsider', 0, '2026-10-01T00:00:00Z', 'out')
outsider.revisions['revisions/outsider.json'].metadata.projectId = 'missing-project'
if (select([project, outsider]).logs.length !== 0) fail('unknown project log')
else console.log('PASS unknown-project-log')

const longTitle = '예시 경계 ' + '가나다라'.repeat(20)
const longProject = pair('long', entry('p-long', 'long-title', 'project', 'r-long'), revision('r-long', 'p-long', longTitle, { type: 'project' }))
const longResult = select([longProject])
if (longResult.projects[0]?.title !== longTitle) fail('long title changed')
else console.log('PASS long-title-kept')

const site = loadPublicProjects()
const publicSlugs = site.logs.map((log) => log.slug).join(',')
if (site.projects.length !== 1 || publicSlugs !== 'example-planning,example-implementation,example-verification') fail(`public order ${publicSlugs}`)
else if (site.projects[0].slug !== 'example-memo-desk' || site.projects[0].topic !== null || site.projects[0].github !== null || site.projects[0].judgment !== '') fail('public project fields')
else if (site.logs[0].next?.slug !== 'example-implementation' || site.logs[2].previous?.slug !== 'example-implementation') fail('public neighbors')
else if (site.logs[0].createdAt < site.logs[1].createdAt) fail('public date order matched read order')
else if (!site.projects[0].related.some((item) => item.href === '/logs/example-planning')) fail('related log link')
else console.log('PASS public-project-selection')

function walk(dir, acc = []) {
  if (!existsSync(dir)) return acc
  for (const name of readdirSync(dir)) {
    const full = path.join(dir, name)
    if (statSync(full).isDirectory()) walk(full, acc)
    else acc.push(full)
  }
  return acc
}

const dist = path.join(root, 'apps', 'web', 'dist')
const htmlFiles = walk(dist).filter((file) => file.endsWith('.html'))
if (htmlFiles.length === 0) fail('dist html missing')

function htmlAt(parts) {
  const file = path.join(dist, ...parts)
  if (!existsSync(file)) {
    fail(`missing ${parts.join('/')}`)
    return ''
  }
  return readFileSync(file, 'utf8')
}

const list = htmlAt(['projects', 'index.html'])
const description = htmlAt(['projects', 'example-memo-desk', 'index.html'])
const logList = htmlAt(['projects', 'example-memo-desk', 'logs', 'index.html'])
const planning = htmlAt(['logs', 'example-planning', 'index.html'])
const implementation = htmlAt(['logs', 'example-implementation', 'index.html'])
const verification = htmlAt(['logs', 'example-verification', 'index.html'])
const home = htmlAt(['index.html'])
const longTitleText = JSON.parse(readFileSync(path.join(root, 'fixtures', 'content', 'boundary', 'long-title.json'), 'utf8')).title
const nested = path.join(dist, 'projects', 'example-memo-desk', 'logs', 'example-planning')

function canonicalHref(html) {
  return html.match(/<link rel="canonical" href="([^"]*)"/)?.[1] ?? ''
}

if (existsSync(nested)) fail('duplicate log path')
if (canonicalHref(list) !== '/projects') fail('list canonical')
if (list && !list.includes('href="/projects/example-memo-desk"')) fail('list card link')
if (list && !list.includes('주제가 아직 없다.')) fail('empty topic')
if (canonicalHref(description) !== '/projects/example-memo-desk') fail('description canonical')
if (description && !description.includes('GitHub 주소가 아직 없다.')) fail('empty github')
if (description && !description.includes('이 구역의 문장이 아직 없다.')) fail('empty judgment')
if (description && !description.includes('href="/logs/example-planning"')) fail('description related log')
if (canonicalHref(logList) !== '/projects/example-memo-desk/logs') fail('log list canonical')
if (logList && logList.indexOf('예시: 기획 기록') > logList.indexOf('예시: 구현 기록')) fail('log list read order')
if (logList && !(logList.indexOf('2026-10-20') < logList.indexOf('2026-10-01') && logList.indexOf('2026-10-01') < logList.indexOf('2026-10-10'))) fail('log list date display')
if (canonicalHref(planning) !== '/logs/example-planning') fail('planning canonical')
if (planning && (!planning.includes('이전 과정이 없다.') || !planning.includes('href="/logs/example-implementation"'))) fail('planning pager')
if (canonicalHref(verification) !== '/logs/example-verification') fail('verification canonical')
if (verification && (!verification.includes('다음 과정이 없다.') || !verification.includes('href="/logs/example-implementation"'))) fail('verification pager')
if (implementation && (!implementation.includes('href="/logs/example-planning"') || !implementation.includes('href="/logs/example-verification"'))) fail('implementation pager')
if (planning && !planning.includes('href="/projects/example-memo-desk"')) fail('return to project')
if (home && !home.includes('href="/projects/example-memo-desk"')) fail('home project link')
if (home && !home.includes('href="/logs/example-planning"')) fail('home log link')

const styled = [list, description, logList, planning].join('\n')
if (!styled.includes('overflow-wrap:anywhere') && !styled.includes('overflow-wrap: anywhere')) fail('long title wrap')

for (const html of [list, description, logList, planning, implementation, verification, home]) {
  if (!html) continue
  if ((html.match(/<main\b/g) ?? []).length !== 1) fail('main count')
  if (html.includes(longTitleText)) fail('boundary title on page')
  if (html.includes('https://example.com')) fail('placeholder origin')
  if (html.includes('/logs/example-planning/logs')) fail('nested log url')
}

if (failed) process.exit(1)
console.log('PASS project-pages')
process.exit(0)
