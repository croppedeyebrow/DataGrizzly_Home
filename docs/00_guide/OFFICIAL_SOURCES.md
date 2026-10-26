# 공식 자료와 확인 범위

확인일: 2026-10-25. 설치 절차와 도구의 책임을 확인한 참고입니다. 특정 최신 버전 숫자를 영구 정답으로 고정하지 않습니다. 실제 선택 버전은 `docs/setup/versions.md`에 기록했습니다.

| 영역 | 공식 자료 | 확인할 내용 |
|---|---|---|
| WSL | https://learn.microsoft.com/en-us/windows/wsl/install | Windows 요구사항·설치·배포판·재부팅 |
| Docker Windows | https://docs.docker.com/desktop/setup/install/windows-install/ | 현재 지원 OS·WSL backend·설치 |
| Node | https://nodejs.org/en/download | 선택 버전·Linux 아키텍처·다운로드 |
| Yarn Security | https://yarnpkg.com/features/security | 설치 스크립트·age gate·Hardened Mode·audit |
| Yarn Workspaces | https://yarnpkg.com/features/workspaces | workspace 선언·조회·실행 |
| Astro | https://docs.astro.build/en/install-and-setup/ | Node 요구·생성·개발 서버 |
| Vite | https://vite.dev/guide/ | Node 요구·React 템플릿·빌드 |
| Spring Boot | https://docs.spring.io/spring-boot/system-requirements.html | Java·Gradle 호환 범위 |
| Kotlin | https://kotlinlang.org/docs/gradle-configure-project.html | Kotlin Gradle Plugin·Gradle 호환 범위 |
| React | https://react.dev/versions | 안정 버전 |
| TypeScript | https://github.com/microsoft/TypeScript/releases | Astro 호환 안정 버전 |
| PostgreSQL | https://www.postgresql.org/support/versioning/ | 지원 기간·유지보수 릴리스 정책 |
| Spring Initializr | https://start.spring.io | 실제 Boot/Kotlin/Gradle 프로젝트 생성 |
| CMake | https://cmake.org/cmake/help/latest/guide/tutorial/index.html | 타깃·out-of-source build·테스트 |
| Rust | https://rust-lang.org/tools/install/ | rustup·OS별 설치 |
| Go | https://go.dev/doc/install | 다운로드·설치·버전 확인 |

추가 구현 참고: PostgreSQL 공식 이미지 https://hub.docker.com/_/postgres , Git https://git-scm.com/doc , VS Code WSL https://code.visualstudio.com/docs/remote/wsl . 이 추가 링크의 세부 버전·동작은 적용 시 다시 확인합니다.

이 문서의 폴더 구조·포트·우선순위·완료조건은 DataGrizzly의 설계 결정입니다. 공식 문서가 이 프로젝트 구조를 요구한다는 의미가 아닙니다. 설치와 제품 구현은 사용자 PC에서 아직 실행·검증하지 않았습니다.
