/**
 * 기능: 백업과 복원 판단.
 * 목적: 같은 서버 디스크만 백업이라 부르지 않고, 운영 데이터베이스를 복원 대상으로 쓰지 않는다.
 * 연관: `entry_denylist`, 릴리스 `manifest.json`, `docs/runbooks/restore.md`.
 */

/**
 * 기능: 백업에 포함할 대상.
 * 목적: 데이터베이스, 첨부, 철회 목록, 릴리스 매니페스트를 함께 보존한다.
 * 연관: `missingMedia`.
 */
export function backupTargets() {
  return ['database', 'media', 'entry_denylist', 'release_manifest', 'workflow_checkpoint']
}

/**
 * 기능: 복원 위치가 운영과 다른지 본다.
 * 목적: 검증 복원이 운영 원본을 덮지 않게 한다.
 * 연관: `datagrizzly` 개발 데이터베이스와 격리 데이터베이스 `datagrizzly_m2`.
 */
export function restoreAllowed({ sourceDb, targetDb, sourceRoot, targetRoot }) {
  if (!sourceDb || !targetDb || sourceDb === targetDb) return { ok: false, code: 'same_database' }
  if (!sourceRoot || !targetRoot || pathKey(sourceRoot) === pathKey(targetRoot)) return { ok: false, code: 'same_disk' }
  return { ok: true, code: 'isolated' }
}

/**
 * 기능: 스냅샷이 가리키는데 없는 첨부.
 * 목적: 데이터베이스만 복구해서 파일이 빠진 경우를 찾는다.
 * 연관: `revision_media`.
 */
export function missingMedia(referenced, present) {
  const have = new Set(present)
  return referenced.filter((id) => !have.has(id))
}

/**
 * 기능: 이미지 복귀와 스키마 복귀를 나눈다.
 * 목적: 데이터 손실이 되는 스키마 롤백을 자동으로 실행하지 않는다.
 * 연관: `deployPlan`.
 */
export function rollbackDecision(kind) {
  if (kind === 'image') return { automatic: false, allowedWhenCompatible: true }
  return { automatic: false, allowedWhenCompatible: false }
}

/**
 * 기능: 복구 목표와 측정값을 비교한다.
 * 목적: RTO 4시간과 RPO 24시간은 목표이고, 측정이 없으면 달성으로 두지 않는다.
 * 연관: `evaluations/search/m5-04.json`의 측정 구분.
 */
export function recoveryComparison({ restoreMinutes, dataAgeHours }) {
  const measured = Number.isFinite(restoreMinutes) && Number.isFinite(dataAgeHours)
  return {
    rtoTargetHours: 4,
    rpoTargetHours: 24,
    measured,
    met: measured && restoreMinutes <= 240 && dataAgeHours <= 24,
  }
}

function pathKey(value) {
  return String(value).replaceAll('\\', '/').replace(/\/+$/, '').toLowerCase()
}
