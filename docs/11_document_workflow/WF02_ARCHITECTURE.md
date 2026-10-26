# WF02 프로그램·프로세스·저장 경계

## 모노레포 추가 경로

| 경로 | 책임 |
|---|---|
| workers/document-workflow/pyproject.toml, uv.lock | Python 전용 의존성·잠금 |
| workers/document-workflow/src/dg_workflow/cli.py | --once / --drain / --job 실행 진입점 |
| src/dg_workflow/adapters/ | 문서 읽기·Kotlin HTTP·Rust subprocess |
| src/dg_workflow/graph/ | 상태 타입·노드·조건부 edge·interrupt |
| src/dg_workflow/checkpoints/ | PostgresSaver 구성·버전 호환 검사 |
| services/cms/.../ingestion/ | 작업 API·권한·lease·승인·멱등 저장 |
| apps/admin/.../imports/ | A13~A15 화면 |
| contracts/ingestion/ | JSON Schema·예시·오류 코드 |
| tests/fixtures/ingestion/ | 합성·공개 허용 자료만 |

위 경로는 구현 대상이며 현재 코드가 존재한다는 뜻이 아닙니다.

## 실행 경계

브라우저는 Kotlin에만 접속합니다. Python은 Kotlin 내부 API를 호출하여 작업을 claim합니다. 작업 큐는 PostgreSQL ingestion_jobs이며 별도 브로커는 없습니다. 최초에는 수동 워커 실행, 운영에서는 timer/cron이 실행 중복 없이 --drain을 호출하는 방식을 선택합니다. LangGraph가 워커를 자동 기동해 주지는 않습니다.

Python에서 기존 Kotlin API로 콘텐츠를 저장합니다. 체크포인트만 전용 DB 권한으로 씁니다. CMS 원본 테이블 직접 수정 권한은 주지 않습니다. 기존 PostgreSQL 인스턴스를 공유하되 langgraph 전용 schema 또는 같은 인스턴스의 별도 DB를 쓰고 search_path/권한을 통합 테스트합니다.

## Rust 실행

OS에 맞는 버전 고정 CLI를 별도 빌드하고 JSON stdin/stdout으로 호출합니다. Python 설치 때 Rust를 자동 빌드하지 않습니다. Windows 개발에서는 기존 WSL Rust 실행 어댑터를 사용하고 인자로 임의 shell 문자열을 조합하지 않습니다. Linux CI/운영은 동일 계약의 Linux 바이너리를 사용합니다. timeout·출력 크기·종료 코드를 검사합니다.

## 권한 분리

일반 관리자는 자신의 작업을 생성·검토합니다. 워커 credential은 claim/heartbeat/artifact/result API와 공개 검색만 호출합니다. 세션 쿠키·DB 관리자 계정·발행 권한을 전달하지 않습니다. resume 요청은 Kotlin이 관리자 권한과 reviewVersion을 검사한 후 command로 저장합니다.

[[WF03_STATE_MACHINE]] [[WF04_DATA_API]] [[WF06_OPERATIONS]]
