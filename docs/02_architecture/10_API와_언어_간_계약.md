# 10. API와 언어 간 계약

> v5.0 기준: [[SCOPE_v5]] 적용. 주력 Kotlin·Rust·TypeScript·PostgreSQL. LLM·C++ 제외, Go는 국소 배포 도구로 필요할 때만 사용합니다. 로컬 자원은 [[02_RESOURCE_BUDGET]] 참조.
HTTP·JSON·파일 계약을 먼저 고정

## 관리 API

POST /api/admin/entries: 생성. PUT /entries/{id}/draft: expectedVersion과 저장. GET /entries/{id}/preview: 초안. POST /entries/{id}/publications: revisionId·예약 시각·idempotency key. GET /publication-jobs/{id}: 상태. POST /entries/{id}/restore: 새 초안. POST /entries/{id}/unpublish: 철회.

## 공개 API

GET /api/search?q=&type=&project=: 근거 검색. 생성형 질문 API는 v4.0에서 제외합니다.

## 오류 계약

401/403 인증·권한, 404 공개 대상 없음, 409 저장 버전/slug 충돌, 422 본문 검증, 429 요청 제한, 503 검색 사용 불가. 검색 장애는 CMS 저장 오류와 분리합니다.

## Rust·Kotlin 파일 계약

Document JSON은 schemaVersion, entryId, revisionId, title, sections, canonicalUrl을 가집니다. Rust 출력 chunk는 chunkId·headingPath·anchor·text·contentHash를 포함합니다. 공개 검색 데이터 manifest에 releaseId·chunkerVersion·indexVersion을 기록합니다.

## 예시 응답

검색 결과 항목: entryId, revisionId, chunkId, title, snippet, url, score. score는 신뢰 확률로 표시하지 않습니다. 원문 링크는 검증된 공개 문서 ID와 절 앵커로 생성합니다.


## v5.0 검색 계약

공개 GET/api/search는Kotlin→PostgreSQL repository로실행합니다. RustDocument/chunk파일계약은본문추출·export에유지합니다. 별도내부C++HTTP검색계약은사용하지않습니다. [[SCOPE_v5]]

## v5.0 자료 워크플로 반영

Python↔Kotlin 작업 API와 Python↔Rust Document JSON은 [[WF04_DATA_API]]가 정의합니다. 승인/lease/version/멱등 key를 생략하지 않습니다.
