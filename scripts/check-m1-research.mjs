/**
 * 개념: 노트와 리서치 렌더가 경계 fixture를 공개 페이지에 섞지 않는지 본다.
 * 목적: 코드와 수식이 실행되지 않고, 논문 주장과 뉴스 날짜가 구분되는지 확인한다.
 * 사용처: scripts/check.ps1, scripts/check.sh 의 프로젝트 페이지 검사 다음 단계.
 */
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { renderDocument, toPreviewHtml } from '../packages/block-renderer/render.mjs'
import { loadPublicResearch, selectPublicResearch } from './public-research.mjs'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
let failed = false

function fail(message) {
  failed = true
  console.log(`FAIL ${message}`)
}

function readJson(name) {
  return JSON.parse(readFileSync(path.join(root, 'fixtures', 'content', name), 'utf8'))
}

const missing = loadPublicResearch(path.join(root, 'fixtures', 'content-missing'))
if (!missing.loadError || missing.notes.length !== 0) fail('missing research fixture')
else console.log('PASS missing-research-empty')

const foreign = selectPublicResearch({ dataset: 'other', public: [] }, {}, {})
if (foreign.notes.length !== 0 || foreign.loadError) fail('foreign dataset')
else console.log('PASS foreign-research-dataset')

const unknown = renderDocument([{ type: 'widget', text: '<script>alert(1)</script>' }])
if (unknown.ok || !unknown.errors.some((error) => error.startsWith('unknown-block'))) fail('unknown block silent')
else if (!toPreviewHtml(unknown).includes('지원하지 않는 블록이다.')) fail('unknown block message')
else console.log('PASS unknown-block')

const duplicate = renderDocument([
  { type: 'heading', level: 2, text: '하나', anchor: 'intro' },
  { type: 'heading', level: 2, text: '둘', anchor: 'intro' },
])
if (duplicate.ok) fail('duplicate anchor published')
else console.log('PASS duplicate-anchor')

const hostileCode = renderDocument([{ type: 'code', language: 'html', text: '<script>alert(1)</script>' }])
const codeHtml = toPreviewHtml(hostileCode)
if (codeHtml.includes('<script>') || !codeHtml.includes('&lt;script&gt;') || !codeHtml.includes('파일명이 아직 없다.')) fail('code escaped')
else console.log('PASS code-escaped')

const hostileMath = renderDocument([{ type: 'math', tex: '\\input{evil}<script>' }])
const mathHtml = toPreviewHtml(hostileMath)
if (mathHtml.includes('\\input') || mathHtml.includes('<script>') || !mathHtml.includes('허용되지 않아')) fail('math rejected')
else console.log('PASS math-rejected')

const plainMath = renderDocument([{ type: 'math', tex: 'a+b' }])
if (plainMath.blocks[0]?.state !== 'shown' || !toPreviewHtml(plainMath).includes('a+b')) fail('plain math')
else console.log('PASS plain-math')

const links = renderDocument([{ type: 'paragraph', text: 'javascript:alert(1) 와 https://arxiv.org/abs/1706.03762' }])
const linkHtml = toPreviewHtml(links)
if (linkHtml.includes('href="javascript') || !linkHtml.includes('href="https://arxiv.org/abs/1706.03762"')) fail('url handling')
else console.log('PASS url-handling')

const image = renderDocument(
  [{ type: 'image', alt: '없는 예시 그림', url: 'https://example.com/fixtures/media/absent-example.svg' }],
  new Set(),
)
const imageHtml = toPreviewHtml(image)
if (!image.blocks[0]?.missing || imageHtml.includes('https://example.com') || !imageHtml.includes('없는 예시 그림')) fail('missing image')
else console.log('PASS missing-image')

const table = renderDocument(readJson('boundary/table.json').blocks)
if (!toPreviewHtml(table).includes('class="table-scroll"')) fail('table scroll')
else console.log('PASS table-scroll')

const longCode = readJson('boundary/long-code.json')
const longRendered = renderDocument(longCode.blocks)
if ((longRendered.blocks[0]?.text ?? '').length < 1000) fail('long code trimmed')
else if (!toPreviewHtml(longRendered).includes('class="code-block"')) fail('long code block')
else console.log('PASS long-code')

const badEntry = {
  entryId: 'n-bad',
  slug: 'bad-note',
  type: 'note',
  example: true,
  publishedRevisionId: 'r-bad',
  updatedAt: '2026-10-01T00:00:00Z',
}
const badRevision = {
  revisionId: 'r-bad',
  title: '나쁜 노트',
  example: true,
  createdAt: '2026-10-01T00:00:00Z',
  metadata: { type: 'note' },
  blocks: [{ type: 'widget' }],
}
const blank = selectPublicResearch({
  dataset: 'public-example',
  public: [
    { role: 'note', entry: 'entries/bad.json', revision: 'revisions/bad.json' },
    { role: 'note', entry: 'entries/blank.json', revision: 'revisions/blank.json' },
  ],
}, {
  'entries/bad.json': badEntry,
  'entries/blank.json': { ...badEntry, entryId: 'n-blank', slug: '', publishedRevisionId: 'r-blank' },
}, {
  'revisions/bad.json': badRevision,
  'revisions/blank.json': { ...badRevision, revisionId: 'r-blank' },
})
if (blank.notes.length !== 0 || blank.rejected.length !== 1) fail('bad note published')
else console.log('PASS unpublished-bad-note')

const site = loadPublicResearch()
const note = site.notes[0]
const paper = site.research.find((item) => item.kind === 'paper')
const news = site.research.find((item) => item.kind === 'news')
const sourceHeading = paper?.blocks.find((block) => block.anchor === 'source')
if (site.notes.length !== 1 || site.research.length !== 2) fail('public counts')
else if (note?.projectId !== null || note?.slug !== 'example-reading-order') fail('note project')
else if (paper?.reproduction !== '재현하지 않았다.' || !paper.claim.includes('적지 않는다')) fail('paper sections')
else if (!news?.eventDate.includes('2023-09-14') || !news?.publicationDate.includes('2023-09-14')) fail('news dates')
else if (news.eventDate === news.publicationDate) fail('news sections collapsed')
else if (!sourceHeading || sourceHeading.headingPath?.join('/') !== 'source') fail('heading path')
else console.log('PASS public-research-selection')

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

function htmlAt(parts) {
  const file = path.join(dist, ...parts)
  if (!existsSync(file)) {
    fail(`missing ${parts.join('/')}`)
    return ''
  }
  return readFileSync(file, 'utf8')
}

function canonicalHref(html) {
  return html.match(/<link rel="canonical" href="([^"]*)"/)?.[1] ?? ''
}

const notes = htmlAt(['notes', 'index.html'])
const notePage = htmlAt(['notes', 'example-reading-order', 'index.html'])
const research = htmlAt(['research', 'index.html'])
const paperPage = htmlAt(['research', 'example-attention-citation', 'index.html'])
const newsPage = htmlAt(['research', 'example-postgresql-16', 'index.html'])
const home = htmlAt(['index.html'])
const boundaryTitles = ['long-title.json', 'long-code.json', 'table.json', 'math.json', 'missing-image.json']
  .map((name) => readJson(`boundary/${name}`).title)

if (canonicalHref(notes) !== '/notes') fail('notes canonical')
if (canonicalHref(notePage) !== '/notes/example-reading-order') fail('note canonical')
if (notePage && !notePage.includes('이 글은 프로젝트에 속하지 않는다.')) fail('note without project')
if (canonicalHref(research) !== '/research') fail('research canonical')
if (canonicalHref(paperPage) !== '/research/example-attention-citation') fail('paper canonical')
if (paperPage && (!paperPage.includes('id="claim"') || !paperPage.includes('id="interpretation"') || !paperPage.includes('id="reproduction"') || !paperPage.includes('재현하지 않았다.'))) fail('paper blocks')
if (paperPage && !paperPage.includes('href="https://arxiv.org/abs/1706.03762"')) fail('paper source link')
if (canonicalHref(newsPage) !== '/research/example-postgresql-16') fail('news canonical')
if (newsPage && (!newsPage.includes('id="event-date"') || !newsPage.includes('id="publication-date"'))) fail('news anchors')
if (home && (!home.includes('href="/notes/example-reading-order"') || !home.includes('href="/research/example-attention-citation"'))) fail('home research links')

const pages = [notes, notePage, research, paperPage, newsPage, home]
for (const html of pages) {
  if (!html) continue
  if ((html.match(/<main\b/g) ?? []).length !== 1) fail('main count')
  if (html.includes('https://example.com')) fail('placeholder origin')
  if (boundaryTitles.some((title) => html.includes(title))) fail('boundary title')
  if (html.includes('이 줄은 실행하지 않는 예시 코드다.')) fail('boundary code published')
}
if (notePage && !notePage.includes('overflow-x:auto') && !notePage.includes('overflow-x: auto')) fail('horizontal scroll')
else if (!failed) console.log('PASS research-pages')

if (failed) process.exit(1)
process.exit(0)
