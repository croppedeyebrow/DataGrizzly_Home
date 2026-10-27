/**
 * 개념: 빌드된 홈과 소개가 공통 틀과 예시 경계를 지키는지 본다.
 * 목적: 예시가 경력처럼 보이지 않고, 경계용 긴 제목이 공개 페이지에 섞이지 않게 한다.
 * 사용처: scripts/check.ps1, scripts/check.sh 의 공개 웹 빌드 다음 단계.
 */
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { emptyHome, loadPublicHome, selectPublicHome } from './public-examples.mjs'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
let failed = false

function fail(message) {
  failed = true
  console.log(`FAIL ${message}`)
}

function readDist(name) {
  return readFileSync(path.join(root, 'apps', 'web', 'dist', name), 'utf8')
}

const missing = loadPublicHome(path.join(root, 'fixtures', 'content-missing'))
if (!missing.loadError || missing.project !== null) fail('missing fixture did not stay empty')
else console.log('PASS missing-fixture-empty')

const empty = selectPublicHome({ dataset: 'other', public: [] }, {})
if (empty.project !== null || empty.logs.length !== 0 || empty.contact !== null) fail('empty selection')
else console.log('PASS empty-selection')

const homeData = loadPublicHome()
if (!homeData.project || homeData.logs.length !== 3 || homeData.contact !== null) fail('public home selection')
else console.log('PASS public-home-selection')

const home = readDist('index.html')
const about = readDist(path.join('about', 'index.html'))
const longTitle = JSON.parse(readFileSync(path.join(root, 'fixtures', 'content', 'boundary', 'long-title.json'), 'utf8')).title

for (const [name, html] of [['home', home], ['about', about]]) {
  const mains = html.match(/<main\b/g) ?? []
  if (mains.length !== 1) fail(`${name} main count ${mains.length}`)
  if (!html.includes('href="#main"')) fail(`${name} skip link`)
  if (!html.includes('id="main"')) fail(`${name} main id`)
  if (!html.includes('aria-current="page"')) fail(`${name} current menu`)
  if (html.includes(longTitle)) fail(`${name} includes boundary title`)
  if (/\/(lab|labs|experiments|signup|register)\b/.test(html)) fail(`${name} forbidden route`)
}

if (!home.includes('예시이며 실제 성과가 아니다.')) fail('home example flag')
if (!home.includes('연락 링크는 아직 없다.')) fail('home empty contact')
if (!about.includes('홈의 예시를 경력으로 쓰지 않는다.')) fail('about career guard')
if (!home.includes(homeData.project.title)) fail('home project title')

if (emptyHome(true).loadError !== true) fail('emptyHome')

if (failed) process.exit(1)
console.log('PASS layout-home')
process.exit(0)
