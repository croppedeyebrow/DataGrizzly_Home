/**
 * 기능: 이미지 승격 판단.
 * 목적: 태그 이름이 아니라 sha256 digest가 같을 때만 다음 환경으로 넘긴다.
 * 연관: `infra/ops/deploy.mjs`, `.github/workflows/ci.yml`. 푸시 자체는 이 함수 밖에서 한다.
 */

/**
 * 기능: digest가 같은지 본다.
 * 목적: `latest` 같은 태그만으로 같은 이미지라고 보지 않는다.
 * 연관: 배포 순서.
 */
export function promote({ digest, builtDigest }) {
  const shape = /^sha256:[a-f0-9]{64}$/
  if (!shape.test(digest ?? '') || !shape.test(builtDigest ?? '')) return { ok: false, code: 'digest_required', exitCode: 1 }
  if (digest !== builtDigest) return { ok: false, code: 'digest_mismatch', exitCode: 1 }
  return { ok: true, code: 'promoted', digest, exitCode: 0 }
}
