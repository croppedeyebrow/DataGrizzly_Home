# WF03 상태·중단·재개·중복 실행

## 업무 상태

| 상태 | 진입·다음 전이 |
|---|---|
| QUEUED | 접수 완료. 워커 claim → RUNNING |
| RUNNING | 처리 중. 검토점 → NEEDS_REVIEW, 일시오류 → RETRY_WAIT, 영구오류 → FAILED |
| NEEDS_REVIEW | 승인/수정 command → RESUME_QUEUED, 거절 → REJECTED |
| RESUME_QUEUED | 워커 claim 후 저장된 command를 적용 → RUNNING |
| RETRY_WAIT | nextAttemptAt 이후 claim → RUNNING; 최대 3회 소진 → FAILED |
| DRAFT_SAVED | Kotlin이 초안 ID를 확정한 종료 상태 |
| FAILED | 사용자 명시 재시도만 허용, 입력 변경 시 새 작업 |
| REJECTED / CANCELLED | 종료. 재개 불가, 다시 시도하려면 새 작업 |

취소는 현재 활성 상태에서 cancelRequestedAt을 기록하며 처리 중에는 다음 안전 지점에서 종료합니다. 초안 저장 트랜잭션이 이미 완료됐다면 DRAFT_SAVED를 반환하고 취소를 거절합니다. 초안을 자동 삭제하지 않습니다.

## 그래프

load → normalize → inspect → retrieve → review(interrupt) → save_draft. inspect는 중복 또는 메타데이터 누락 시 review payload에 이슈를 넣습니다. 관련 글 조회가 실패하면 경고를 표시하고 검토로 진행할 수 있지만 원문 추출 실패는 승인할 수 없습니다. 수정 가능한 것은 메타데이터·발췌·코멘트·관련 글이며 원본 교체는 새 작업입니다.

state에는 schemaVersion, workflowVersion, jobId, sourceHash, artifactId, sectionRefs, metadata, issues, relatedEntryIds, reviewVersion, draftId를 저장합니다. 원문 전체·비밀번호·토큰은 state에 넣지 않습니다. thread_id는 jobId로 고정합니다.

## lease와 fencing

claim은 Kotlin의 단일 트랜잭션에서 실행 가능한 행을 잠그고 leaseToken, leaseUntil, attempt를 갱신합니다. 최초 제안 lease 120초, heartbeat 30초. 만료된 RUNNING은 다음 claim에서 회수합니다. 모든 워커 결과 쓰기는 최신 leaseToken 일치와 유효기간을 검사합니다. 이전 워커의 늦은 결과는 409로 거절합니다. 워커 한 개 정책이어도 이 규칙은 유지합니다.

## 승인 재개

검토 승인 API는 reviewVersion과 commandId를 저장하고 RESUME_QUEUED로 전환합니다. 워커가 꺼져 있어도 이 요청은 유지됩니다. 워커는 같은 thread_id와 Command(resume=...)로 재개하며 처리 성공 후 command를 완료 처리합니다. 재시작 시 checkpoint의 interrupt/next와 업무 상태를 대조하여 누락된 NEEDS_REVIEW 상태를 복구합니다. 상태·체크포인트 간 분산 트랜잭션은 가정하지 않습니다.

## 멱등성과 crash window

interrupt가 있는 노드는 재개 시 처음부터 실행될 수 있으므로 그 앞에 초안 생성 같은 부수 효과를 넣지 않습니다. review 노드는 검토 데이터 제시만 합니다. save_draft에는 jobId 기반 고정 idempotency key와 입력 hash를 보냅니다. DB 저장 후 응답 유실 또는 체크포인트 실패 시 같은 요청을 재전송해 같은 draftId를 받습니다. 동일 key에 다른 본문이면 409입니다.

배포 시 기존 미완료 작업의 workflowVersion을 유지하거나 명시적 호환 변환을 제공합니다. 그래프 node 이름·state schema를 임의 변경하여 대기 작업을 깨뜨리지 않습니다.

[[WF04_DATA_API]] [[WF07_TESTS]]
