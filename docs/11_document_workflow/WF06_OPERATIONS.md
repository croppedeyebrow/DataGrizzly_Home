# WF06 로컬 자원·실행·배포·복구

## 실제 PC 적용

Ryzen 5 7530U, RAM 조회 13.8GiB, C: 여유 23.4GiB 기준입니다. [[02_RESOURCE_BUDGET]]의 공간 게이트를 먼저 적용합니다. Python은 S20 이전에 설치하지 않으며 모델·torch·transformers·OCR 패키지를 기본 의존성으로 추가하지 않습니다.

WORKFLOW 모드: PostgreSQL + Kotlin API + Python 워커 1개 + 관리자 UI 1개. Rust는 사전 빌드된 CLI만 실행합니다. 공개 웹 watcher·전체 빌드·Rust 컴파일은 중지합니다. Python RSS 512MiB를 관측 목표로 시작하되 보장하지 않습니다. 총 시스템 메모리 85% 지속·paging·UI 지연이면 UI를 정적 빌드로 바꾸거나 작업 입력을 줄입니다. 워커 컨테이너 사용 시 768MiB/CPU1부터 측정하고 OOM은 영구 성공으로 처리하지 않습니다.

## 실행 의미

--once: 실행 가능한 작업 하나를 claim하고 완료/검토대기/실패 후 종료합니다. --drain: 실행 가능한 작업을 순차 처리하고 없으면 종료합니다. --job ID: 지정 작업도 동일한 claim/권한/lease 절차를 거칩니다. 사람이 검토하는 동안 프로세스를 띄워 기다리지 않습니다.

로컬에서는 관리자 등록 후 직접 CLI를 실행합니다. 승인 후 다시 CLI를 실행해야 초안 저장이 진행됩니다. UI는 ‘워커 실행 대기’를 명확히 표시합니다. 운영에서 자동 처리가 필요하면 호스트 timer가 1분 간격으로 --drain을 호출하고 프로세스 lock과 DB lease로 중복 실행을 막습니다. timer는 구현·배포해야 하며 LangGraph가 제공하는 자동 기동으로 설명하지 않습니다.

## CI·컨테이너

경로별 CI: Python lint/type/unit, Kotlin 계약/DB, Rust fixture, 관리자 상태 UI. 네트워크 fetch 테스트는 통제된 fixture 서버를 사용합니다. 별도 nightly 모델 테스트는 없습니다. Python 이미지는 필요한 의존성만 lock 기반 설치하고 비루트로 실행합니다. Rust 바이너리 호환 OS/아키텍처를 검사하고 이미지 digest를 기록합니다. 기본 compose up에 워커를 상시 추가하지 말고 workflow profile의 one-shot service로 제공합니다.

## 보존과 복구

제안: 종료 작업의 원문·중간 artifact·checkpoint 30일, 안전한 작업 이벤트 90일. 승인으로 생성한 CMS 초안은 CMS 보존 정책을 따릅니다. NEEDS_REVIEW는 30일 알림 후 명시적 만료 처리하고 그 뒤 유예 7일을 두며 대기 작업을 조용히 삭제하지 않습니다. 이 수치는 설정 가능하며 삭제 전 대상 목록 dry-run을 제공합니다.

백업은 업무 DB·체크포인트 DB/schema·비공개 artifact 저장소를 함께 대상으로 합니다. 복원 후 활성 lease를 만료시키고 artifact hash/존재, job↔checkpoint↔receipt를 대조합니다. draft receipt가 있으면 재생성하지 않습니다. checkpoint만 복원해도 원문이 복구된다고 가정하지 않습니다.

## 관측·롤백

jobId·stage·duration·attempt·safe error code·queue age·worker lastSeen 기록. 원문은 제외합니다. 워커가 없어도 CMS 장애로 표시하지 않습니다. 실패율과 대기 시간을 별도로 관측합니다. 배포 롤백 시 checkpoint schema/workflowVersion 호환을 먼저 검사하고 비호환 작업은 격리합니다. 미완료 작업을 자동 삭제하지 않습니다.
