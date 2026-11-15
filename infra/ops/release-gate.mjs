/**
 * 기능: 출시 전 점검.
 * 목적: 모델이 꺼진 상태를 결함으로 보지 않고, 공개 전환과 검증 완료를 구분한다.
 * 연관: `docs/03_features/F15_배포_운영_복구.md`, `evaluations/search/m5-04.json`.
 */

/**
 * 기능: 예산 알림과 지출 상한을 나눈다.
 * 목적: 알림이 자동으로 비용을 막았다고 적지 않는다.
 * 연관: 출시 점검의 운영 설정.
 */
export function budgetSignal(kind) {
  if (kind === 'hard_cap') return { notifies: true, stopsSpend: true }
  return { notifies: true, stopsSpend: false }
}

/**
 * 기능: 출시 게이트.
 * 목적: 공개 URL과 실측 복구가 없으면 출시 완료로 닫지 않는다.
 * 연관: M5-04의 미측정 지연, M6-03의 미측정 복구.
 */
export function releaseGate({ searchLive, backupMeasured, publicUrl }) {
  return {
    schemaVersion: 1,
    modelOffIsDefect: false,
    highAvailability: false,
    launchComplete: false,
    checks: [
      { id: 'model', passed: true, note: '모델이 꺼져 있는 것은 결함이 아니다' },
      { id: 'search-live', passed: searchLive === true, note: '검색 지연과 메모리의 실측' },
      { id: 'backup-measured', passed: backupMeasured === true, note: '격리 복원의 실측' },
      { id: 'public-url', passed: Boolean(publicUrl), note: '실제 공개 주소와 릴리스 표시' },
    ],
    singleServer: '한 대에서 API와 데이터베이스가 함께 멈추면 공개 읽기와 관리 화면이 같이 멈춘다',
  }
}
