/**
 * 개념: 브라우저 사전 검사 로더.
 * 실행: 제한 시간 안에 모듈을 불러오지 못하면 null을 돌려준다.
 * 목적: 모듈이 없거나 늦어도 편집기의 저장은 서버 검사를 그대로 사용한다.
 * 관련: `apps/admin/src/editor/Editor.tsx`의 자동 저장, `services/api/src/content.rs`, `tools/content-core`.
 */
export async function loadOptionalCheck<T>(load: () => Promise<T>, timeoutMs: number): Promise<T | null> {
  let timer: ReturnType<typeof setTimeout> | undefined
  try {
    return await Promise.race([
      load(),
      new Promise<T>((_resolve, reject) => {
        timer = setTimeout(() => reject(new Error('timeout')), timeoutMs)
      }),
    ])
  } catch {
    return null
  } finally {
    if (timer) clearTimeout(timer)
  }
}
