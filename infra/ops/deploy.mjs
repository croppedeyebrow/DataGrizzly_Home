/**
 * 기능: 배포 순서.
 * 목적: 마이그레이션 확인 뒤에 이미지를 올리고, 스키마는 실패해도 자동으로 되돌리지 않는다.
 * 연관: `infra/ops/promote.mjs`, `services/api/migrations`.
 */

/**
 * 기능: 다음 동작을 고른다.
 * 목적: 호환되지 않는 스키마는 배포 전에 멈추고, 스모크 실패는 이미지만 되돌릴 후보로 둔다.
 * 연관: `promote`.
 */
export function deployPlan({ migrationCompatible, smokeOk }) {
  if (!migrationCompatible) {
    return { action: 'stop', rollbackSchema: false, rollbackImage: false, reason: 'migration_incompatible' }
  }
  if (!smokeOk) {
    return { action: 'stop', rollbackSchema: false, rollbackImage: true, reason: 'smoke_failed' }
  }
  return { action: 'done', rollbackSchema: false, rollbackImage: false, reason: 'ok' }
}

/**
 * 기능: 데이터베이스 포트가 루프백에만 있는지 본다.
 * 목적: compose 파일의 데이터베이스가 외부 주소로 열리지 않았는지 확인한다.
 * 연관: `infra/local/compose.yaml`.
 */
export function databaseLoopbackOnly(composeText) {
  const ports = [...String(composeText).matchAll(/^\s*-\s*"([^"]+)"/gm)].map((match) => match[1])
  const opened = ports.filter((port) => !port.startsWith('127.0.0.1:'))
  return { public: opened.length > 0, ports }
}
