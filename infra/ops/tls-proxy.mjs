/**
 * 기능: 로컬 TLS 역방향 프록시.
 * 실행: TLS_PFX와 TLS_PFX_PASSWORD를 환경에 두고 `node infra/ops/tls-proxy.mjs`
 * 목적: 127.0.0.1에서만 TLS로 API를 연다. 공개 인증서나 공개 도메인은 만들지 않는다.
 * 연관: `services/api`의 `/actuator/health`, `infra/ops/journey.mjs`.
 */
import { readFileSync } from 'node:fs'
import http from 'node:http'
import https from 'node:https'

const pfx = readFileSync(process.env.TLS_PFX ?? '')
const passphrase = process.env.TLS_PFX_PASSWORD ?? ''
const server = https.createServer({ pfx, passphrase }, (request, response) => {
  const upstream = http.request(
    {
      hostname: '127.0.0.1',
      port: 8080,
      path: request.url,
      method: request.method,
      headers: { ...request.headers, host: '127.0.0.1:8080' },
    },
    (answered) => {
      response.writeHead(answered.statusCode ?? 502, answered.headers)
      answered.pipe(response)
    },
  )
  upstream.on('error', () => {
    if (!response.headersSent) response.writeHead(502)
    response.end('{"error":"unavailable"}')
  })
  request.pipe(upstream)
})
server.listen(8443, '127.0.0.1')
