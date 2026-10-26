# WF07 테스트·완료 증거

테스트 대상은 구현된 기능이며 현재 문서 작성만으로 통과한 것으로 표시하지 않습니다.

| ID | 시나리오 | 기대 결과 |
|---|---|---|
| W01 | 정상 PDF·Markdown | 출처·페이지/절·정규화 결과가 fixture와 일치 |
| W02 | 이미지 PDF·손상·과대 파일 | 명시적 오류, 무한 재시도·자동 OCR 없음 |
| W03 | 같은 원문 재등록 | 기존 작업/자료 안내, 다른 내용 덮어쓰기 없음 |
| W04 | 추출 중 프로세스 kill | lease 회수 후 복구, artifact hash 일관 |
| W05 | interrupt 직후 종료 | 재기동 후 같은 검토 화면, 신규 thread 생성 안 함 |
| W06 | 워커 종료 상태에서 승인 | command 보존, 재실행 후 초안 한 개 |
| W07 | 초안 commit 후 응답 유실 | 같은 key 재전송 → 동일 entry/revision |
| W08 | 오래된 lease의 늦은 결과 | 409, 최신 상태 유지 |
| W09 | 승인 더블클릭·오래된 검토창 | 같은 command 멱등, 다른 stale version은 409 |
| W10 | 취소와 저장 경합 | 먼저 확정한 트랜잭션에 따라 종료, 저장된 초안 삭제 안 함 |
| W11 | 다른 관리자 jobId 접근 | 403/404, 원문·작업 정보 노출 없음 |
| W12 | URL redirect·IPv6·DNS 재바인딩 | 내부 주소 접근 차단, 실패 시 fetch 비활성 유지 |
| W13 | HTML/Markdown XSS | script 실행 없음, source는 텍스트로 표시 |
| W14 | 원문·초안 공개 검색 | 결과·snippet·RSS·sitemap에 미노출 |
| W15 | 전체 백업 복원 | 대기 작업 재개, receipt 기준 중복 방지 |
| W16 | Python 프로세스 없음 | 공개 글·CMS 수동 작성·일반 검색 정상 |
| W17 | workflowVersion 변경 | 구버전 대기 작업 호환 또는 명시적 격리 |

단위: parser/상태 분기/입력 검증. 통합: 실제 PostgreSQL·Kotlin HTTP·durable checkpointer. E2E: 등록→검토→초안→별도 발행. 외부 사이트 live 호출을 CI 필수 테스트로 삼지 않습니다.

각 결과에는 커밋·명령·OS·버전·fixture·기대/실제·최대 RSS·미검증 항목을 남깁니다. 자원 측정은 작은 fixture와 한도 근처 fixture를 구분합니다.
