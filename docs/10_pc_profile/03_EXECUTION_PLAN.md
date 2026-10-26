# v5.0 실제 진행 순서 — 주력4개 기술

| 순서 | 태스크 | 실행 범위 |
|---|---|---|
| A | S00~S05 | PC·디스크·OS 확인, Windows 에디터·Git·빈 저장소 |
| B | S06→S09→S10→S19 | Node/TypeScript/Astro만 초기화. 초기 검증 |
| C | M0→M1 | API 없이 fixture로 공개 웹과 계약 구현 |
| D | S07→S08→S11~S14→S18→M2 | JDK·관리자·Kotlin·DB를 추가하고 저장/편집 구현 |
| E | M3 | 발행·철회. watcher를 종료한 뒤 정적 빌드 |
| F | 공간게이트→S16→M4 | Rust 콘텐츠 도구. Ubuntu 설치는 이 시점 |
| G | M5 | PostgreSQL 공개 검색 데이터와 Kotlin 검색 API |
| H | M6 | 기존 CI/Compose/스크립트로 배포·복구·출시 |

S15(C++), M7(LLM)은 이번 범위에서 제외합니다. S17(Go)은 M6 중 필요가 입증된 경우만 수행합니다. M4-04 WASM은 선택이며 초기 출시를 막지 않습니다. 기존 ID는 이전 대화와의 연결을 위해 남겨두고 상태를 '범위 제외' 또는 '조건부 보류'로 표시합니다.

S19 초기 완료는 공개웹·실제 존재하는 패키지의 재현 검증입니다. JDK·DB는 M2 직전, Rust는 M4 직전에 검증합니다. M0-03은 미래 언어 설치를 요구하지 않습니다.

## Windows / Linux 경계

Windows 기본 루트 C:\Dev\sideproject\My_Web에서 Node·JDK를 실행합니다. docker-desktop에 로그인해 코드를 개발하지 않습니다. 사용자 PowerShell 확인 결과 개발용 Ubuntu는 없고 `docker-desktop`만 등록되어 있습니다. Ubuntu는 M4/S16에서 Linux Rust 검증이 실제로 필요할 때만 설치합니다. 설치하면 ~/dev/datagrizzly-native에는 Git 소스와 Rust crate만 두고 node_modules·Gradle cache·모델은 복사하지 않습니다. Linux Rust 바이너리는 Windows에서 직접 실행할 수 없으므로 wsl.exe 호출 또는 CI Linux 경로로 실행합니다. 실제 배포는 Linux에서 별도 검증합니다.

공간이 확보되지 않으면 M4를 로컬 실행 미검증으로 보류할 수 있습니다. CI 실행이 가능하면 같은fixture로 Rust를 검증하되 사용자 계정/사용량을 확인하고 로컬 완료라고 쓰지 않습니다. 추가 클라우드 지출을 자동 전제하지 않습니다.

## v5.0 자료 워크플로 반영

M0~M6 기반 완성 후 [[S20]]→[[S21]]→[[S22]]→[[WF08_TASK_MAP]]의 M8 순서로 확장합니다. S15/M7은 계속 제외합니다.
