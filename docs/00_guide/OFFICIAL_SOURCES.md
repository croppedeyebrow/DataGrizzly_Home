# 공식 자료와 확인 범위

확인일: 2026-10-25. 설치 절차와 도구의 책임을 확인한 참고입니다. 특정 최신 버전 숫자를 영구 정답으로 고정하지 않습니다. 실제 선택 버전은 `docs/setup/versions.md`에 기록했습니다.

| 영역 | 공식 자료 | 확인할 내용 |
|---|---|---|
| WSL | https://learn.microsoft.com/en-us/windows/wsl/install | Windows 요구사항·설치·배포판·재부팅 |
| Docker Windows | https://docs.docker.com/desktop/setup/install/windows-install/ | 현재 지원 OS·WSL backend·설치 |
| Node | https://nodejs.org/en/download | 선택 버전·Linux 아키텍처·다운로드 |
| Yarn Security | https://yarnpkg.com/features/security | 설치 스크립트·age gate·Hardened Mode·audit |
| Yarn Workspaces | https://yarnpkg.com/features/workspaces | workspace 선언·조회·실행 |
| Vite | https://vite.dev/guide/ | Node 요구·React·빌드·미리보기 |
| React | https://react.dev/versions | 안정 버전 |
| TypeScript | https://github.com/microsoft/TypeScript/releases | 프로젝트에 고정한 안정 버전 |
| Tailwind CSS | https://tailwindcss.com/docs/installation/using-vite | Vite 연동·버전 |
| Rust | https://rust-lang.org/tools/install/ | rustup·OS별 설치 |
| Axum | https://docs.rs/axum/ | HTTP 서버 |
| PostgreSQL | https://www.postgresql.org/support/versioning/ | 지원 기간·유지보수 릴리스 정책 |
| CMake | https://cmake.org/cmake/help/latest/guide/tutorial/index.html | 타깃·out-of-source build·테스트 |
| Go | https://go.dev/doc/install | 다운로드·설치·버전 확인 |

추가 구현 참고: PostgreSQL 공식 이미지 https://hub.docker.com/_/postgres , Git https://git-scm.com/doc , VS Code WSL https://code.visualstudio.com/docs/remote/wsl . 이 추가 링크의 세부 버전·동작은 적용 시 다시 확인합니다.

이 문서의 폴더 구조·포트·우선순위·완료조건은 DataGrizzly의 설계 결정입니다. 공식 문서가 이 프로젝트 구조를 요구한다는 의미가 아닙니다. 2026-11-01 현재 스택의 공식 참고는 위 표의 Vite, React, TypeScript, Tailwind, Rust, Axum, PostgreSQL이다. 고정 버전은 `docs/setup/versions.md`에 있다.
