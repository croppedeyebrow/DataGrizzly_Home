/**
 * 개념: 후보 빌드를 검사 결과로 멈춘다.
 * 실행: CLI 종료 코드가 0일 때만 true다. 1과 빈 값은 false다.
 * 목적: 검사에 실패한 수정본으로 후보 폴더를 만들지 않는다.
 * 관련: `infra/publish/build-candidate.mjs`, `tools/content-cli`.
 */
export function buildMayContinue(status) {
  return status === 0
}
