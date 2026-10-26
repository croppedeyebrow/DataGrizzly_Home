# Rust 콘텐츠 처리와 선택적 Go

> v5.0 기준: [[SCOPE_v5]] 적용. 주력 Kotlin·Rust·TypeScript·PostgreSQL. LLM·C++ 제외, Go는 국소 배포 도구로 필요할 때만 사용합니다. 로컬 자원은 [[02_RESOURCE_BUDGET]] 참조.
Rust core는공통블록스키마검증·절/anchor추출·텍스트정규화·JSONL/Markdown내보내기를담당합니다. Kotlin발행작업이RustCLI의버전·exitcode·manifest를검사하고결과를DB검색행과정적빌드에사용합니다. 실패하면발행을차단합니다. 공개미리보기와fixture/anchor규칙을공유합니다.

초기Windows개발시Rust는공간확보후Ubuntu전용checkout에서빌드합니다. Kotlin은wsl.exe어댑터또는LinuxCI에서실행하며Linux실행파일을Windows에서직접실행하지않습니다. 운영은Linux에서검증합니다.

Go는현재필수가아닙니다. CI/Compose/작은스크립트로배포검사를수행하고필요할때만좁은CLI로추가합니다. [[M6-01]] [[S17]]

## v5.0 자료 워크플로 반영

자료 워커는 Rust 정규화·검증 CLI를 재사용합니다. LangChain 어댑터가 같은 기능을 다시 구현하지 않습니다. [[WF02_ARCHITECTURE]]
