# Rust 콘텐츠 처리와 선택적 Go

> v5.0 기준: [[SCOPE_v5]] 적용. 주력 React·TypeScript·Tailwind·Rust·PostgreSQL. LLM·C++ 제외, Go는 국소 배포 도구로 필요할 때만 사용합니다. 로컬 자원은 [[02_RESOURCE_BUDGET]] 참조.
Rust 라이브러리는 공통 블록 스키마 검증, 절과 anchor 추출, 텍스트 정규화, JSONL과 Markdown 내보내기를 담당합니다. 발행을 맡은 같은 Rust API가 그 결과의 버전과 manifest를 검사하고 DB 검색 행과 정적 빌드에 사용합니다. 실패하면 발행을 막습니다. 공개 미리보기와 fixture, anchor 규칙을 공유합니다.

로컬 API는 이 PC의 Windows Rust로 빌드합니다. Ubuntu는 로컬 API의 선행 조건이 아닙니다. 운영 바이너리는 Linux에서 따로 검증합니다.

Go는현재필수가아닙니다. CI/Compose/작은스크립트로배포검사를수행하고필요할때만좁은CLI로추가합니다. [[M6-01]] [[S17]]

## v5.0 자료 워크플로 반영

자료 워커는 Rust 정규화·검증 라이브러리를 재사용합니다. LangChain 어댑터가 같은 기능을 다시 구현하지 않습니다. [[WF02_ARCHITECTURE]]
