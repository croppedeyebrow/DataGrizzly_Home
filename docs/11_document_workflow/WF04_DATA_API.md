# WF04 DB·API·파일 계약

## 소유권과 테이블

Kotlin migration이 아래 업무 테이블을 관리합니다. LangGraph 패키지의 checkpoint 테이블은 전용 schema/DB에서 패키지 setup 절차로 관리하며 직접 구조를 흉내 내지 않습니다.

| 테이블 | 필수 필드·제약 |
|---|---|
| ingestion_sources | id, owner_id, kind, private_object_key, sha256, size, mime, original_url, fetched_at, metadata_json; owner+sha256 중복 안내 |
| ingestion_jobs | id, owner_id, source_id, status, version, workflow_version, thread_id(unique), attempt, next_attempt_at, lease_token, lease_until, cancel_requested_at, draft_id, error_code, timestamps |
| ingestion_artifacts | id, job_id, stage, artifact_version, object_key, content_hash, size; unique(job_id,stage,artifact_version) |
| ingestion_commands | command_id(unique), job_id, expected_review_version, action, sanitized_payload, status; 활성 검토 버전당 승인 한 개 |
| ingestion_events | id, job_id, event_type, safe_details, created_at; 토큰·원문 미기록 |
| ingestion_draft_receipts | job_id(unique), idempotency_key(unique), payload_hash, entry_id, revision_id |

source/artifact는 기존 비공개 미디어 저장소를 재사용하고 공개 CDN 경로에 두지 않습니다. 외부 원문은 별도 공개 첨부로 자동 전환하지 않습니다.

## 관리자 API

| API | 동작 |
|---|---|
| POST /api/admin/imports | sourceId 또는 허용 URL과 requestId 접수, 202 jobId/status |
| GET /api/admin/imports | 상태별 페이지 목록; 소유권 필터 |
| GET /api/admin/imports/{id} | 상태·현재 단계·reviewVersion·허용 action·결과 |
| POST /api/admin/imports/{id}/review | commandId, expectedReviewVersion, action=approve/reject, corrections; 202 재개 대기 |
| POST /api/admin/imports/{id}/retry | 재시도 가능 FAILED만 QUEUED 전환 |
| POST /api/admin/imports/{id}/cancel | expectedVersion으로 취소 의도 저장 |

업로드는 기존 media API를 통해 비공개 sourceId를 받습니다. 브라우저가 지정한 임의 로컬 경로·object key는 신뢰하지 않습니다.

## 내부 워커 API

POST /internal/imports/claim → 작업 없음 204, 있으면 job/lease/command/artifact 참조. POST /{id}/heartbeat, POST /{id}/artifacts, POST /{id}/review-ready, POST /{id}/failure, POST /{id}/draft를 정의합니다. 내부 경로의 접두사는 /internal/imports입니다. claim 외 요청은 jobId·leaseToken 필수입니다. claim 때 사용자 입력을 다시 권한 검사합니다. artifact 읽기는 job 범위 인증 또는 단기 서명 URL로 제한합니다.

POST /{id}/draft는 수집 원문 저장이 아닌 검토 완료한 초안 저장입니다. Kotlin이 승인 command·버전·소유권·payload hash를 검사하고 entry/revision/receipt/job DRAFT_SAVED를 한 트랜잭션으로 저장합니다. 중복 요청은 receipt를 조회합니다. Python의 성공 표시에만 의존하지 않습니다.

## Document 계약

JSON: schemaVersion, sourceId, sourceHash, title(nullable), authors[], publishedAt(nullable), doi(nullable), sections[{sectionId,page,heading,text,sourceLocator}], warnings[]. PDF는 page, Markdown은 heading/line 범위를 보존합니다. Rust가 정규화하고 sectionId를 결정합니다. LangChain Document.page_content에는 결과 텍스트, metadata에는 sourceId/sectionId/locator를 넣습니다.

Retriever는 Kotlin GET /api/search를 호출하고 공개 entryId/revisionId/title/snippet/url을 Document로 변환합니다. 관련 글 조회에 관리자 초안 API를 사용하지 않습니다. 인용 없는 생성 문장을 만들지 않습니다.

## 오류

401/403 인증·소유권, 409 stale version/lease/key conflict, 413 크기, 415 형식, 422 본문 추출 불가/잘못된 승인, 429 요청 한도, 503 일시 장애. UI에는 복구 행동과 jobId를 표시하고 stack trace나 원문 비밀을 보여주지 않습니다.

## 검토 내용의 무결성

review-ready는 추출 artifact hash와 검토 가능 필드를 고정합니다. 승인 command는 그 hash와 reviewVersion에 묶고 Kotlin이 정제한 corrections를 저장합니다. 초안 저장 시 워커의 임의 본문을 그대로 믿지 않고 승인된 필드·발췌 sectionId·사용자 코멘트와 대조합니다. 체크포인트의 승인 값만으로 저장 권한을 만들지 않습니다.

업무 상태의 재시도는 입력 sourceHash를 바꾸지 않습니다. 자료 내용 교체는 새 source/job입니다. 거절 command는 Kotlin에서 종료 상태를 확정하며 워커는 이미 종료된 작업을 resume하지 않습니다.
