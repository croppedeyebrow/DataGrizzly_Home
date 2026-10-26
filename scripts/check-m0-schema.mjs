import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import Ajv2020 from 'ajv/dist/2020.js'
import { parse } from 'yaml'

const Ajv = Ajv2020.default ?? Ajv2020
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const fixtureRoot = path.join(root, 'fixtures', 'contracts')
const requiredOperations = [
  '/api/admin/entries post',
  '/api/admin/entries/{id}/draft put',
  '/api/admin/entries/{id}/preview get',
  '/api/admin/entries/{id}/publications post',
  '/api/admin/publication-jobs/{id} get',
  '/api/admin/entries/{id}/restore post',
  '/api/admin/entries/{id}/unpublish post',
  '/api/search get',
]

function pointers(errors) {
  return (errors ?? []).map((error) => {
    const base = error.instancePath || ''
    if (error.keyword === 'required' && error.params?.missingProperty) {
      return `${base}/${error.params.missingProperty}`
    }
    return base || '/'
  })
}

function structureProblems(document) {
  const problems = []
  const anchors = new Set()
  for (const block of document.blocks ?? []) {
    if (block?.anchor) {
      if (anchors.has(block.anchor)) problems.push(`duplicate_anchor:${block.anchor}`)
      anchors.add(block.anchor)
    }
    if (block?.type === 'table' && Array.isArray(block.header) && Array.isArray(block.rows)) {
      for (const row of block.rows) {
        if (Array.isArray(row) && row.length !== block.header.length) {
          problems.push('table_width')
        }
      }
    }
  }
  const serialized = JSON.stringify(document)
  if (serialized.toLowerCase().includes('<script')) problems.push('script_text')
  return problems
}

function referenceResult(document, knownProjectIds) {
  const projectId = document?.metadata?.projectId
  if (typeof projectId !== 'string') return 'skip'
  return knownProjectIds.includes(projectId) ? 'pass' : 'fail'
}

function lintOpenApi(spec) {
  const problems = []
  if (spec.openapi !== '3.1.0') problems.push('openapi_version')
  if (spec.servers?.[0]?.url !== '/') problems.push('server_url')
  const serverText = JSON.stringify(spec.servers ?? [])
  if (serverText.includes('127.0.0.1') || serverText.includes('example.com')) {
    problems.push('concrete_server')
  }
  if (spec.paths?.['/api/ask']) problems.push('ask_api')
  if (!spec.components?.securitySchemes?.ownerSession) problems.push('owner_session')
  if (!spec.components?.schemas?.Error) problems.push('error_schema')
  if (spec.components?.parameters?.Limit?.schema?.maximum !== 50) problems.push('limit')
  const found = []
  for (const [route, item] of Object.entries(spec.paths ?? {})) {
    for (const method of ['get', 'post', 'put', 'delete']) {
      const operation = item?.[method]
      if (!operation) continue
      found.push(`${route} ${method}`)
      if (!operation.responses || Object.keys(operation.responses).length === 0) {
        problems.push(`responses:${route}:${method}`)
      }
    }
  }
  for (const operation of requiredOperations) {
    if (!found.includes(operation)) problems.push(`missing:${operation}`)
  }
  return problems
}

const schema = JSON.parse(await readFile(path.join(root, 'contracts', 'content.schema.json'), 'utf8'))
const manifest = JSON.parse(await readFile(path.join(fixtureRoot, 'manifest.json'), 'utf8'))
const openapi = parse(await readFile(path.join(root, 'contracts', 'openapi.yaml'), 'utf8'))
const ajv = new Ajv({ allErrors: true, strict: false })
ajv.addSchema(schema)
const validators = new Map()
let failed = false

function fail(message) {
  failed = true
  console.log(`FAIL ${message}`)
}

const openApiProblems = lintOpenApi(openapi)
if (openApiProblems.length === 0) {
  console.log('PASS openapi-structure')
} else {
  fail(`openapi-structure ${openApiProblems.join(',')}`)
}

if (manifest.schemaVersion !== 1) fail('manifest schemaVersion')

for (const testCase of manifest.cases) {
  const filePath = path.join(fixtureRoot, testCase.file)
  const document = JSON.parse(await readFile(filePath, 'utf8'))
  if (testCase.file !== 'invalid/missing-schema-version.json' && !Object.hasOwn(document, 'schemaVersion')) {
    fail(`${testCase.file} missing schemaVersion field`)
    continue
  }
  if (testCase.file.startsWith('valid/') && document.example !== true) {
    fail(`${testCase.file} example flag`)
  }
  if (testCase.file === 'valid/entry.json') {
    if (document.draftRevisionId === document.publishedRevisionId) {
      fail('entry pointers are not separate')
    } else {
      console.log('PASS entry-pointers-distinct')
    }
  }

  if (!validators.has(testCase.definition)) {
    validators.set(
      testCase.definition,
      ajv.compile({ $ref: `${schema.$id}#/$defs/${testCase.definition}` }),
    )
  }
  const validate = validators.get(testCase.definition)
  const schemaOk = validate(document) === true
  const schemaResult = schemaOk ? 'pass' : 'fail'
  const foundPaths = pointers(validate.errors)
  if (schemaResult !== testCase.schema) {
    fail(`${testCase.file} schema expected ${testCase.schema} got ${schemaResult} paths=${foundPaths.join('|')}`)
  } else if (testCase.errorPath && !foundPaths.includes(testCase.errorPath)) {
    fail(`${testCase.file} errorPath expected ${testCase.errorPath} got ${foundPaths.join('|')}`)
  } else {
    console.log(`PASS ${testCase.file} schema=${schemaResult}`)
  }

  if (testCase.structure !== 'skip') {
    const problems = schemaOk ? structureProblems(document) : ['skipped']
    const structureResult = problems.length === 0 ? 'pass' : 'fail'
    if (structureResult !== testCase.structure) {
      fail(`${testCase.file} structure expected ${testCase.structure} got ${structureResult} ${problems.join(',')}`)
    } else {
      console.log(`PASS ${testCase.file} structure=${structureResult}`)
    }
  }

  if (testCase.references !== 'skip') {
    const referencesResult = schemaOk ? referenceResult(document, manifest.knownProjectIds) : 'skip'
    if (referencesResult !== testCase.references) {
      fail(`${testCase.file} references expected ${testCase.references} got ${referencesResult}`)
    } else {
      console.log(`PASS ${testCase.file} references=${referencesResult} source=fixture-list`)
    }
  }
}

if (failed) process.exit(1)
console.log('PASS schema-fixtures')
process.exit(0)
