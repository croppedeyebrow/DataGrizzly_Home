import { readFileSync, statSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import Ajv2020 from 'ajv/dist/2020.js'

const Ajv = Ajv2020.default ?? Ajv2020
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const contentRoot = path.join(root, 'fixtures', 'content')
const schema = JSON.parse(readFileSync(path.join(root, 'contracts', 'content.schema.json'), 'utf8'))
const manifest = JSON.parse(readFileSync(path.join(contentRoot, 'manifest.json'), 'utf8'))
const ajv = new Ajv({ allErrors: true, strict: false })
ajv.addSchema(schema)
const validateRevision = ajv.compile({ $ref: `${schema.$id}#/$defs/revisionDocument` })
const validateEntry = ajv.compile({ $ref: `${schema.$id}#/$defs/entry` })
let failed = false

function fail(message) {
  failed = true
  console.log(`FAIL ${message}`)
}

function readJson(relativePath) {
  return JSON.parse(readFileSync(path.join(contentRoot, relativePath), 'utf8'))
}

function anchorsOf(document) {
  return new Set((document.blocks ?? []).filter((block) => block.anchor).map((block) => block.anchor))
}

function requireAnchors(document, label, names) {
  const anchors = anchorsOf(document)
  for (const name of names) {
    if (!anchors.has(name)) fail(`${label} missing anchor ${name}`)
  }
}

function mediaName(url) {
  return new URL(url).pathname.split('/').pop()
}

function mediaExists(url) {
  const name = mediaName(url)
  try {
    statSync(path.join(root, 'fixtures', 'media', name))
    return true
  } catch {
    return false
  }
}

if (manifest.schemaVersion !== 1) fail('manifest schemaVersion')
if (manifest.dataset !== 'public-example') fail('dataset name')

const publicPaths = new Set()
const entryIds = new Set()
const loaded = []

for (const item of manifest.public) {
  if (item.revision.includes('boundary') || item.entry.includes('boundary')) {
    fail(`${item.role} public path includes boundary`)
  }
  publicPaths.add(item.revision)
  const revision = readJson(item.revision)
  const entry = readJson(item.entry)
  if (validateRevision(revision) !== true) fail(`${item.revision} schema ${ajv.errorsText(validateRevision.errors)}`)
  if (validateEntry(entry) !== true) fail(`${item.entry} schema ${ajv.errorsText(validateEntry.errors)}`)
  if (revision.example !== true || entry.example !== true) fail(`${item.role} example flag`)
  if (!revision.title.startsWith('예시')) fail(`${item.role} title is not marked as an example`)
  if (!JSON.stringify(revision).includes('실제 성과가 아니다')) fail(`${item.role} missing example disclaimer`)
  if (entry.entryId !== revision.entryId) fail(`${item.role} entry id`)
  if (entry.publishedRevisionId !== revision.revisionId) fail(`${item.role} published pointer`)
  if (entry.draftRevisionId === entry.publishedRevisionId) fail(`${item.role} draft pointer`)
  if (entry.type !== item.role) fail(`${item.role} entry type`)
  if (revision.metadata.type !== (item.role === 'log' ? 'log' : item.role)) fail(`${item.role} metadata type`)
  entryIds.add(entry.entryId)
  loaded.push({ ...item, revision, entry })
}

const project = loaded.find((item) => item.role === 'project')
if (!project) fail('project missing')
if (project && project.entry.entryId !== manifest.projectId) fail('project id')
if (project) {
  requireAnchors(project.revision, 'project', ['problem', 'goal', 'scope', 'choice', 'status', 'limit'])
  if (project.revision.metadata.projectId) fail('project metadata has projectId')
}

const logs = loaded.filter((item) => item.role === 'log')
if (logs.length !== 3) fail(`log count ${logs.length}`)
const stages = logs.map((item) => item.revision.metadata.stage).sort()
if (stages.join() !== 'implementation,planning,verification') fail(`stages ${stages.join()}`)
for (const log of logs) {
  if (log.revision.metadata.projectId !== manifest.projectId) fail(`${log.entry} projectId`)
}
const byDate = [...logs].sort((a, b) => a.revision.createdAt.localeCompare(b.revision.createdAt))
const byOrder = [...logs].sort((a, b) => a.revision.metadata.displayOrder - b.revision.metadata.displayOrder)
if (byDate.map((item) => item.revision.metadata.stage).join() === byOrder.map((item) => item.revision.metadata.stage).join()) {
  fail('date order matches read order')
} else if (byOrder.map((item) => item.revision.metadata.displayOrder).join() !== '0,1,2') {
  fail('displayOrder is not 0,1,2')
} else {
  console.log(`PASS read-order ${byOrder.map((item) => item.revision.metadata.stage).join(',')}`)
  console.log(`PASS date-order ${byDate.map((item) => item.revision.metadata.stage).join(',')}`)
}

const note = loaded.find((item) => item.role === 'note')
if (!note) fail('note missing')
if (note && note.revision.metadata.projectId) fail('note has projectId')

const paper = loaded.find((item) => item.role === 'paper')
if (!paper) fail('paper missing')
if (paper) {
  requireAnchors(paper.revision, 'paper', ['source', 'authors', 'claim', 'interpretation', 'reproduction'])
  const paperText = JSON.stringify(paper.revision)
  if (!paperText.includes('https://arxiv.org/abs/1706.03762')) fail('paper source url')
  if (!paperText.includes('2017-06-12')) fail('paper date')
  if (!paperText.includes('재현하지 않았다')) fail('paper reproduction marker')
  if (!paperText.includes('테스트 자료')) fail('paper test marker')
}

const news = loaded.find((item) => item.role === 'news')
if (!news) fail('news missing')
if (news) {
  requireAnchors(news.revision, 'news', ['source', 'event-date', 'publication-date'])
  const newsText = JSON.stringify(news.revision)
  if (!newsText.includes('https://www.postgresql.org/about/news/postgresql-16-released-2715/')) {
    fail('news source url')
  }
  if (!newsText.includes('사건일 2023-09-14')) fail('news event date')
  if (!newsText.includes('발표일 2023-09-14')) fail('news publication date')
  if (!newsText.includes('테스트 자료')) fail('news test marker')
}

for (const item of loaded) {
  for (const block of item.revision.blocks ?? []) {
    if (block.type === 'related' && !entryIds.has(block.entryId)) fail(`${item.role} related ${block.entryId}`)
    if (block.type === 'image' && !mediaExists(block.url)) fail(`${item.role} media ${block.url}`)
  }
}

const boundaryPaths = new Set(manifest.boundary.map((item) => item.file))
for (const file of boundaryPaths) {
  if (publicPaths.has(file)) fail(`${file} is in the public dataset`)
}

for (const item of manifest.boundary) {
  const document = readJson(item.file)
  const schemaOk = validateRevision(document) === true
  if (!schemaOk) fail(`${item.file} schema ${ajv.errorsText(validateRevision.errors)}`)
  if (item.expect === 'long-title') {
    if (!(document.title?.length >= 80)) fail('long-title length')
    else console.log(`PASS ${item.file} title=${document.title.length}`)
  } else if (item.expect === 'long-code') {
    const code = (document.blocks ?? []).find((block) => block.type === 'code')
    if (!code || code.text.length < 1000) fail('long-code length')
    else console.log(`PASS ${item.file} code=${code.text.length}`)
  } else if (item.expect === 'table') {
    if (!(document.blocks ?? []).some((block) => block.type === 'table')) fail('table block')
    else console.log(`PASS ${item.file}`)
  } else if (item.expect === 'math') {
    if (!(document.blocks ?? []).some((block) => block.type === 'math')) fail('math block')
    else console.log(`PASS ${item.file}`)
  } else if (item.expect === 'media-missing') {
    const image = (document.blocks ?? []).find((block) => block.type === 'image')
    if (!image) fail('missing-image block')
    else if (mediaExists(image.url)) fail('missing-image file exists')
    else console.log(`PASS ${item.file} media-missing`)
  } else if (item.expect === 'empty-body') {
    if (!Array.isArray(document.blocks) || document.blocks.length !== 0) fail('empty-body')
    else console.log(`PASS ${item.file}`)
  } else if (item.expect === 'unknown-project') {
    if (document.metadata?.projectId === manifest.projectId) fail('bad-reference uses the public project')
    const related = (document.blocks ?? []).find((block) => block.type === 'related')
    if (!related || entryIds.has(related.entryId)) fail('bad-reference related id is public')
    else console.log(`PASS ${item.file} unknown-project`)
  } else {
    fail(`${item.file} unknown expect`)
  }
}

if (failed) process.exit(1)
console.log('PASS content-fixtures')
process.exit(0)
