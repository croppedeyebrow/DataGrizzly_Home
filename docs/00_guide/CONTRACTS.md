# 공통 계약과 불변조건

## 시스템 경계

- Astro: 정적 공개 HTML. React: 관리자 및 필요한 상호작용.
- Kotlin Spring Boot: 인증·CMS·발행·검색 중계. PostgreSQL: 콘텐츠 원본.
- Rust: 콘텐츠 CLI, 선택 WASM. Go: 필요 시 국소 배포 CLI. Python은 자료 처리 워커에만 사용하며 CMS 업무 규칙을 소유하지 않습니다.
- 단일 서버+CDN+외부 백업을 초기안으로 둡니다. 상시 GPU·Kubernetes·Kafka·자체 CI 플랫폼은 제외합니다.

## 원본·수정본·공개 세대

1. Revision 본문은 불변이며 저장할 때 새 수정본을 만듭니다.
2. Entry의 초안 포인터와 공개 포인터는 별개입니다. 복원은 새 초안을 만듭니다.
3. 저장 요청 expectedVersion은 서버의 원자적 비교·갱신으로 처리합니다.
4. SiteRelease는 revision 목록과 콘텐츠/인덱스 manifest를 고정합니다.
5. 발행 작업은 중복 전달될 수 있습니다. idempotency key, lease, fencing/세대 검사를 함께 적용합니다.
6. CDN·DB·검색 사이 전역 트랜잭션은 없습니다. 후보 업로드→검증→활성화 단계를 기록하고 부분 실패를 재조정합니다. 일시적 불일치를 숨기지 않습니다.
7. 철회 denylist는 rollback과 복원 후에도 유지합니다. 공개 응답과 생성 직전에 확인합니다. 이미 외부에 복제된 콘텐츠 회수는 보장하지 않습니다.

## 최소 API 계약 — M0-02에서 확정

기계가 읽는 계약은 `contracts/openapi.yaml`과 `contracts/content.schema.json`이다. 아래 표는 그 명세의 색인이다.

| 접점 | 입력 | 성공 | 실패 |
|---|---|---|---|
| POST /api/admin/entries | type, title | 201 entryId, version | 401/403/422 |
| PUT /api/admin/entries/{id}/draft | expectedVersion, body, metadata | 200 revisionId, version | 409/422 |
| POST /api/admin/entries/{id}/publications | revisionId, idempotencyKey | 202 jobId | 409/422 |
| GET /api/admin/publication-jobs/{id} | 인증 | 200 state, releaseId, error | 401/403/404 |
| POST /api/admin/entries/{id}/restore | revisionId, expectedVersion | 200 새 draft revision | 409/404 |
| POST /api/admin/entries/{id}/unpublish | expectedVersion | 202 jobId | 409 |
| GET /api/search | q, type, project | 200 hits, releaseId, mode | 429/503 |

모든 오류는 code, message, requestId 및 선택 fieldErrors를 사용합니다. 내부 stacktrace는 공개하지 않습니다. 응답의 version·releaseId 의미를 혼용하지 않습니다. 길이/파일/timeout 상한은 환경별 설정으로 확정하고 테스트합니다.

## 파일 계약

Document: schemaVersion, entryId, revisionId, title, sections, canonicalUrl.
Chunk: chunkId, entryId, revisionId, headingPath, anchor, text, contentHash.
Manifest: releaseId, schemaVersion, chunkerVersion, indexVersion, hashes.
모든 producer/consumer는 지원 버전을 검사합니다. 인덱스 검색 점수는 신뢰 확률이 아닙니다.

## 출시 게이트


## 의존성 보완

- M1-01 실행은 M0-03 저장소 준비 후 시작합니다.
- M3-04에서 아직 없는 PostgreSQL 검색은 갱신 hook과 계약까지만 작성하며 실제 연결은 M5에서 합니다.
- M6-04는 원기획 선행 M6-03 외에도 M5-04 평가를 필수로 확인합니다.


## v3.0 초기 환경 경계

필요한 단계의 초기 설정만 실행합니다. C++ S15는 제외하고 Go S17은 조건부입니다. Python은 S20~S22에서 준비합니다. M0-03은 초기 웹 구조를 검증하며 재초기화하지 않습니다. S14는 bootstrap migration만, M2-01은 실제 업무 테이블을 만듭니다. 기본 CI는 S19, 운영 배포 CI/CD는 M6입니다.

## v5.0 확정 변경

[[SCOPE_v5]]를적용합니다. 검색은PostgreSQL공개검색행+KotlinAPI이며RustCLI가발행텍스트를추출합니다. 별도C++검색/추론서버는없습니다. Go는선택입니다. S19/M0-03은현재설치된공개웹만초기검증하고DB는M2전S14에서확인합니다. 실행 절차는 `docs/setup/local-run.md`에 있다. 2026-10-26에 다른 경로의 잠금 파일 빌드와 미리보기가 통과했다. API와 DB는 실행하지 않았다.

## v5.0 자료 워크플로 반영

자료 워크플로 계약은 [[WF04_DATA_API]]가 기준입니다. 작업/승인/초안 저장의 권한과 멱등성을 유지합니다.
