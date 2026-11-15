/**
 * 기능: 릴리스 확인 판단.
 * 목적: 헬스, 릴리스 표시, 매니페스트가 같을 때만 성공으로 본다.
 * 연관: `services/api`의 `/actuator/health`, 후보 폴더의 `release.txt`와 `manifest.json`, `infra/ops/verify-release.mjs`.
 */

/**
 * 기능: 로그에서 비밀값을 지운다.
 * 목적: 실패 출력에 비밀번호가 남지 않게 한다.
 * 연관: `verifyRelease`.
 */
export function redact(text) {
  return String(text ?? '')
    .replace(/(password\s*[=:]\s*)(\S+)/gi, '$1[redacted]')
    .replace(/(POSTGRES_PASSWORD\s*[=:]\s*)(\S+)/g, '$1[redacted]')
}

/**
 * 기능: 확인 결과와 종료 코드.
 * 목적: 시간 초과, 연결 실패, 다른 릴리스는 0이 아닌 코드로 끝낸다.
 * 연관: `verify-release.mjs`.
 */
export function verifyRelease({ healthUp, marker, expectedReleaseId, manifestReleaseId, timedOut, connectionFailed, logText }) {
  const log = redact(logText)
  if (timedOut) return result(false, 'timeout', log)
  if (connectionFailed) return result(false, 'connection', log)
  if (!healthUp) return result(false, 'health', log)
  if (!marker || marker.trim() !== expectedReleaseId) return result(false, 'release_mismatch', log)
  if (manifestReleaseId !== expectedReleaseId) return result(false, 'manifest_mismatch', log)
  return result(true, 'ok', log)
}

function result(ok, code, log) {
  return { schemaVersion: 1, ok, code, exitCode: ok ? 0 : 1, log }
}
