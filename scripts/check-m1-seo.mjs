/**
 * 개념: 탐색 분류와 SEO 산출물이 공개 주소만 쓰는지 본다.
 * 목적: 빈 분류와 잘못된 slug를 구분하고, localhost와 example.com이 canonical과 수집 문서에 들어가지 않게 한다.
 * 사용처: scripts/check.ps1, scripts/check.sh 의 리서치 검사 다음 단계.
 */
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { buildMeta, buildRss, buildSitemap, resolveSiteOrigin } from '../packages/seo/meta.mjs'
import { siteOrigin } from '../packages/seo/site-origin.mjs'
import { loadPublicIndex } from './public-index.mjs'
import { classifySlug, loadPublicTaxonomy, selectPublicTaxonomy } from './public-taxonomy.mjs'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const dist = path.join(root, 'apps', 'web', 'dist')
let failed = false

function fail(message) {
  failed = true
  console.log(`FAIL ${message}`)
}

function readJson(name) {
  return JSON.parse(readFileSync(path.join(root, 'fixtures', 'content', name), 'utf8'))
}

function walk(dir, files = []) {
  for (const name of readdirSync(dir)) {
    const file = path.join(dir, name)
    if (statSync(file).isDirectory()) walk(file, files)
    else files.push(file)
  }
  return files
}

const missing = loadPublicTaxonomy(path.join(root, 'fixtures', 'content-missing'))
if (!missing.loadError || missing.topics.length !== 0 || missing.series.length !== 0) fail('missing taxonomy fixture')
else console.log('PASS missing-taxonomy-empty')

const foreign = selectPublicTaxonomy({ dataset: 'other', taxonomies: { topics: [], series: [] } }, [])
if (foreign.topics.length !== 0 || !foreign.rejected.some((item) => item.reason === 'foreign-dataset')) fail('foreign taxonomy')
else console.log('PASS foreign-taxonomy-dataset')

const note = readJson('entries/note.json')
const noteRevision = readJson('revisions/note.json')
const logs = ['log-planning', 'log-implementation', 'log-verification'].map((name) => ({
  entry: readJson(`entries/${name}.json`),
  revision: readJson(`revisions/${name}.json`),
}))
const publicEntries = [
  { entryId: note.entryId, slug: note.slug, title: noteRevision.title, path: `/notes/${note.slug}`, example: true },
  ...logs.map((item) => ({
    entryId: item.entry.entryId,
    slug: item.entry.slug,
    title: item.revision.title,
    path: `/logs/${item.entry.slug}`,
    example: true,
  })),
]
const selected = selectPublicTaxonomy(
  {
    dataset: 'public-example',
    taxonomies: {
      topics: [
        { slug: 'Topic', title: '잘못된 주제', example: true, entryIds: [note.entryId] },
        { slug: '', title: '빈 slug', example: true, entryIds: [note.entryId] },
        { slug: 'example-empty', title: '빈 주제', example: true, entryIds: ['missing-entry'] },
        { slug: 'example-reading', title: '예시 주제', example: true, entryIds: [note.entryId, note.entryId] },
        { slug: 'example-reading', title: '중복 주제', example: true, entryIds: [note.entryId] },
        { slug: 'hidden-topic', title: '숨긴 주제', example: false, entryIds: [note.entryId] },
      ],
      series: [
        {
          slug: 'example-order',
          title: '예시 순서',
          example: true,
          items: [
            { entryId: logs[2].entry.entryId, position: 2 },
            { entryId: logs[0].entry.entryId, position: 0 },
            { entryId: logs[1].entry.entryId, position: 1 },
            { entryId: 'missing-entry', position: 3 },
          ],
        },
        { slug: 'example-empty-series', title: '빈 시리즈', example: true, items: [{ entryId: 'missing-entry', position: 0 }] },
        { slug: 'bad_series', title: '잘못된 시리즈', example: true, items: [{ entryId: logs[0].entry.entryId, position: 0 }] },
      ],
    },
  },
  publicEntries,
)
const reading = selected.topics.find((topic) => topic.slug === 'example-reading')
const ordered = selected.series.find((series) => series.slug === 'example-order')
if (selected.topics.length !== 1 || !reading || reading.entries.length !== 1 || reading.entries[0].path !== `/notes/${note.slug}`) fail('topic page entries')
else if (classifySlug('topic', 'example-empty', selected) !== 'empty') fail('empty topic')
else if (classifySlug('topic', 'Topic', selected) !== 'invalid-slug' || classifySlug('topic', '', selected) !== 'invalid-slug') fail('invalid topic slug')
else if (classifySlug('topic', 'not-a-topic', selected) !== 'unknown') fail('unknown topic')
else if (classifySlug('topic', 'hidden-topic', selected) !== 'not-example') fail('hidden topic')
else if (classifySlug('series', 'example-empty-series', selected) !== 'empty') fail('empty series')
else if (classifySlug('series', 'bad_series', selected) !== 'invalid-slug') fail('invalid series slug')
else if (!ordered || ordered.entries.map((entry) => entry.slug).join(',') !== logs.map((item) => item.entry.slug).join(',')) fail('series order')
else console.log('PASS taxonomy-boundaries')

const live = loadPublicTaxonomy()
if (live.loadError || live.topics.length !== 0 || live.series.length !== 0) fail('live taxonomy published')
else console.log('PASS live-taxonomy-empty')

const index = loadPublicIndex()
const expected = [
  '/',
  '/about',
  '/projects',
  '/projects/example-memo-desk',
  '/projects/example-memo-desk/logs',
  '/logs/example-planning',
  '/logs/example-implementation',
  '/logs/example-verification',
  '/notes',
  '/notes/example-reading-order',
  '/research',
  '/research/example-attention-citation',
  '/research/example-postgresql-16',
]
const paths = index.records.map((record) => record.path)
if (index.loadError || [...paths].sort().join('|') !== [...expected].sort().join('|')) {
  fail(`public index paths ${paths.join(',')}`)
} else console.log('PASS public-index')

const withTraps = [
  ...index.records,
  { path: '/notes/example-reading-order', title: '중복', kind: 'entry' },
  { path: '/notes/draft-secret', title: '초안 비밀', kind: 'entry', draft: true },
  { path: '/admin/entries', title: '관리', kind: 'page' },
  { path: '/notes?topic=cs', title: '필터', kind: 'page' },
  { path: '/404', title: '없음', kind: 'page' },
]
const explicit = buildSitemap(withTraps, 'https://catalog.invalid')
const locs = [...explicit.xml.matchAll(/<loc>([^<]*)<\/loc>/g)].map((match) => match[1])
const expectedLocs = expected.map((route) => `https://catalog.invalid${route}`).sort()
if (!explicit.absolute || [...locs].sort().join('|') !== expectedLocs.join('|')) fail(`explicit sitemap urls ${locs.join(',')}`)
else if (explicit.xml.includes('draft-secret') || explicit.xml.includes('/admin') || explicit.xml.includes('topic=cs') || explicit.xml.includes('/404')) fail('sitemap exclusion')
else if (explicit.rejected.filter((item) => item.reason === 'duplicate').length !== 1) fail('duplicate url')
else if (explicit.rejected.filter((item) => item.reason === 'draft').length !== 1) fail('draft url')
else console.log('PASS explicit-sitemap')

for (const origin of ['', 'http://127.0.0.1:4321', 'http://localhost:4321', 'https://example.com', 'https://example.com/notes/example']) {
  const blocked = buildSitemap(withTraps, origin)
  const rss = buildRss(withTraps, origin)
  if (blocked.absolute || rss.absolute) fail(`absolute allowed for ${origin || 'empty'}`)
  else if (blocked.xml.includes('127.0.0.1') || blocked.xml.includes('localhost') || blocked.xml.includes('example.com')) fail(`blocked sitemap host ${origin || 'empty'}`)
  else if (rss.xml.includes('127.0.0.1') || rss.xml.includes('localhost') || rss.xml.includes('example.com') || rss.xml.includes('draft-secret')) fail(`blocked rss host ${origin || 'empty'}`)
  else if (blocked.xml.includes('<loc>')) fail(`loc without origin ${origin || 'empty'}`)
}
if (failed) {
  /* 호스트 거부는 위에서 이유를 남긴다. */
} else console.log('PASS blocked-origins')

const localMeta = buildMeta({ title: '홈', description: '설명', path: '/', date: '2026-10-28T00:00:00Z', siteOrigin: 'http://127.0.0.1:4321' })
const placeholderMeta = buildMeta({ title: '홈', description: '설명', path: '/notes/example-reading-order', siteOrigin: 'https://example.com' })
const explicitMeta = buildMeta({ title: '홈', description: '설명', path: '/about', date: 'not-a-date', siteOrigin: 'https://catalog.invalid' })
if (localMeta.canonical !== '/' || localMeta.canonicalIsAbsolute || localMeta.author !== 'DataGrizzly') fail('localhost canonical')
else if (placeholderMeta.canonical !== '/notes/example-reading-order' || placeholderMeta.canonical.includes('example.com')) fail('placeholder canonical')
else if (explicitMeta.canonical !== 'https://catalog.invalid/about' || explicitMeta.date !== '' || explicitMeta.og.type !== 'website') fail('explicit meta')
else if (resolveSiteOrigin(siteOrigin).ok) fail('committed origin is absolute')
else console.log('PASS meta-origin')

const escaped = buildRss([{ path: '/notes/example-reading-order', title: 'A & B <script>', kind: 'entry', updatedAt: '2026-10-26T01:00:00Z' }], 'https://catalog.invalid')
if (!escaped.xml.includes('A &amp; B &lt;script&gt;') || escaped.xml.includes('<script>')) fail('rss escape')
else if (!escaped.xml.includes('<item>') || escaped.xml.includes('/about')) fail('rss entries')
else console.log('PASS rss-entries')

const liveSitemap = buildSitemap(index.records, siteOrigin)
const liveRss = buildRss(index.records, siteOrigin)
if (liveSitemap.absolute || liveRss.items.length !== 0 || !liveRss.xml.includes('절대 링크를 넣지 않았다.')) fail('live feed absolute')
else console.log('PASS live-feed-relative')

if (!existsSync(dist)) fail('dist missing')
else {
  const files = walk(dist)
  const html = files.filter((file) => file.endsWith('.html') || file.endsWith('.xml') || file.endsWith('.txt'))
  const boundaryTitle = readJson('boundary/long-title.json').title
  for (const file of html) {
    const text = readFileSync(file, 'utf8')
    if (text.includes('https://example.com') || text.includes('127.0.0.1') || text.includes('localhost')) fail(`host in ${path.relative(dist, file)}`)
    if (text.includes(boundaryTitle) || text.includes('draft-secret')) fail(`private text in ${path.relative(dist, file)}`)
  }
  const home = readFileSync(path.join(dist, 'index.html'), 'utf8')
  const missingPage = readFileSync(path.join(dist, '404.html'), 'utf8')
  const sitemap = readFileSync(path.join(dist, 'sitemap.xml'), 'utf8')
  const rss = readFileSync(path.join(dist, 'rss.xml'), 'utf8')
  const robots = readFileSync(path.join(dist, 'robots.txt'), 'utf8')
  const notePage = readFileSync(path.join(dist, 'notes', 'example-reading-order', 'index.html'), 'utf8')
  if (!home.includes('name="description"') || !home.includes('name="author" content="DataGrizzly"') || !home.includes('property="og:title"')) fail('home meta')
  else if (!home.includes('rel="canonical" href="/"') || !home.includes('<main id="main"')) fail('home canonical')
  else if (missingPage.includes('rel="canonical"') || !missingPage.includes('페이지를 찾을 수 없다') || !missingPage.includes('href="/projects"')) fail('404 page')
  else if (!missingPage.includes('검색은 아직 연결하지 않았다.') || !missingPage.includes('content="noindex"') || !missingPage.includes('<main id="main"')) fail('404 empty state')
  else if (sitemap.includes('<loc>') || sitemap.includes('example.com') || !sitemap.includes('<urlset')) fail('dist sitemap')
  else if (rss.includes('<item>') || !rss.includes('절대 링크를 넣지 않았다.')) fail('dist rss')
  else if (!robots.includes('Disallow: /admin') || robots.includes('Sitemap:')) fail('robots')
  else if (!notePage.includes('property="article:modified_time"') || !notePage.includes('프로젝트에 속하지 않는 학습 예시')) fail('note html body')
  else if (existsSync(path.join(dist, 'topics')) || existsSync(path.join(dist, 'series'))) fail('empty taxonomy pages')
  else console.log('PASS dist-seo')
}

if (failed) {
  console.log('FAIL seo-pages')
  process.exit(1)
}
console.log('PASS seo-pages')
