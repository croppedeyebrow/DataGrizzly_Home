# S05 단계별 버전 매트릭스

- 확정일: 2026-10-25 (Asia/Seoul)
- 프로젝트 루트: `C:\Dev\sideproject\My_Web`
- 상태: **M0-04 예시 콘텐츠 fixture 통과. 공개 페이지에는 연결하지 않음. API와 DB는 미실행**
- 원칙: 기존 설치를 우선 재사용하고, 프로젝트 의존성은 yarn.lock·Gradle Wrapper·빌드 파일로 정확히 고정한다.
- Git: 사용자 요청에 따라 초기화·설정·커밋·원격 연결을 수행하지 않음

## 결정 요약

### 공개 웹과 관리자

| 구성 | 선택 버전 | 현재 상태 | 결정 |
|---|---:|---|---|
| Node.js | **22.23.4 LTS** | 22.23.4 설치·검증 완료 | 재사용; Windows x64 공식 MSI 설치 |
| npm | **10.9.9** | 설치·검증 완료 | Node 포함 도구로 유지; 프로젝트 패키지 관리에는 사용하지 않음 |
| Corepack | **0.36.0** | 설치·검증 완료 | 프로젝트에 고정한 Yarn 실행기 |
| Yarn | **4.18.1** | S09 실행·검증 완료 | 보안 우선 프로젝트 패키지 관리자 |
| create-astro | **5.2.6** | S10에서 일회성 실행 완료 | 프로젝트 의존성으로 남기지 않음 |
| Astro | **7.4.2** | `apps/web` 설치·정적 빌드 검증 | 공개 웹 버전으로 고정 |
| TypeScript | **6.0.3** | `apps/web` 설치, `astro check` 통과 | Astro와 관리자 workspace에서 통일 |
| `@astrojs/check` | **0.9.12** | `apps/web` 설치, 오류 0 | 공개 웹 타입 검사 |
| React / React DOM | **19.3** | 미설치 | S11 관리자에 사용 |
| create-vite | **9.3.1** | 미설치 | S11에서 일회성 정확 버전 실행 |
| Vite | **8.3.5** | 미설치 | S11 관리자 빌드 도구 |
| `@vitejs/plugin-react` | **6.1.2** | 미설치 | S11 관리자 플러그인 |

S06에서 Node 22.23.2를 동일 LTS 계열의 22.23.4로 패치 업데이트했다. 함께 설치된 npm 10.9.9는 유지하지만 프로젝트 패키지 관리는 Yarn 4.18.1로 통일한다. Yarn은 `node_modules` 호환 방식, 제3자 설치 스크립트 기본 차단, Hardened Mode, 3일 age gate를 사용한다.

TypeScript 최신 채널은 7.0.2지만 Astro 7 계열은 TypeScript 7 type-check를 아직 정식 지원 대상으로 두지 않는다. Astro 7.4.2 자체가 사용하는 안정 버전인 TypeScript 6.0.3을 선택한다.

### Kotlin CMS와 데이터베이스

| 구성 | 선택 버전 | 현재 상태 | 결정 |
|---|---:|---|---|
| JDK | **21.0.8 LTS** | 2026-10-31 `java`/`javac` 21.0.8, x64, `JAVA_HOME` 일치 | 재사용, JVM toolchain 21로 고정. 신규 설치 없음 |
| Spring Boot | **4.1.2** | 프로젝트 미생성 | S12에서 정확 버전 사용 |
| Kotlin Gradle Plugin | **2.4.21** | 전역 CLI 2.0.21 설치 | 프로젝트 플러그인은 2.4.21로 별도 고정 |
| Gradle Wrapper | **9.7.0** | 미생성 | Kotlin 2.4.21의 완전 지원 상한에 맞춤 |
| PostgreSQL | **16.x** | 16.10 설치·실행 중 | 기존 네이티브 서비스 재사용 후보 |
| PostgreSQL 유지보수 목표 | **16.15** | 16.10 | S13 전 백업·릴리스 노트 확인 후 사용자 승인 범위에서 패치 업데이트 검토 |

Spring Boot 4.1.2는 Java 17~27 및 Gradle 8.14+ 또는 9.x를 지원한다. JDK 21은 범위 안이다. Spring Boot의 Kotlin 최소 요구는 2.2.x이며 Kotlin 2.4.21은 충족한다. Kotlin 2.4.21이 완전히 지원하는 Gradle 상한은 9.7.0이므로 최신 Gradle 9.8.2 대신 9.7.0을 선택한다.

전역 `kotlinc` 2.0.21은 간단한 CLI 확인용 설치일 뿐 Spring Boot 프로젝트의 Kotlin 버전을 결정하지 않는다. 실제 빌드는 Gradle Wrapper와 `build.gradle.kts`에 고정된 Kotlin 2.4.21을 사용한다.

PostgreSQL 16은 현재 지원되는 메이저 버전이다. 설치된 16.10은 개발 접속을 수락하지만 최신 16.x 유지보수 릴리스는 16.15다. 데이터 디렉터리를 보존하고 백업 없이 설치 프로그램을 덮어쓰지 않는다.

### 후속·조건부 기술

| 구성 | 현재 버전 | S05 결정 |
|---|---:|---|
| Rust / Cargo | 1.92.0 Windows MSVC | 설치 상태만 기록; M4/S16에서 Linux 대상과 toolchain을 다시 확정 |
| Python | Anaconda Python 3.11.5 | 설치 상태만 기록; S20에서 별도 환경과 프로젝트 버전 결정 |
| Docker Desktop / CLI | 4.43.2 / 28.3.2 | 재설치하지 않음; S08에서 엔진·Compose 검증 |
| Go | 미설치 | M6에서 반복되는 배포 도구 필요가 입증될 때만 도입 |
| C++ / CMake | CMake 미설치 | 범위 제외 |
| LLM·임베딩·벡터DB | 없음 | 범위 제외 |
| Ubuntu | 미설치, WSL에는 `docker-desktop`만 존재 | M4/S16에서 Linux Rust 검증이 필요할 때만 설치 검토 |

## 공식 호환 근거

- Node.js 릴리스: Node 22와 24는 LTS이며 22.23.4가 현재 22.x 패치다. <https://nodejs.org/en/about/previous-releases>
- Astro 7.4.2 패키지는 Node 22.23.4 이상과 npm 9.6.5 이상을 요구한다. <https://registry.npmjs.org/astro/latest>
- Astro 생성기 5.2.6: <https://www.npmjs.com/package/create-astro>
- TypeScript 6.0.3 릴리스: <https://github.com/microsoft/TypeScript/releases>
- React 최신 안정 버전 19.3: <https://react.dev/versions>
- Vite 8.3.5와 create-vite 9.3.1: <https://www.npmjs.com/package/vite> · <https://www.npmjs.com/package/create-vite>
- Spring Boot 4.1.2 시스템 요구사항: <https://docs.spring.io/spring-boot/system-requirements.html>
- Spring Boot Kotlin 지원: <https://docs.spring.io/spring-boot/reference/features/kotlin.html>
- Kotlin 2.4.21과 Gradle 호환표: <https://kotlinlang.org/docs/gradle-configure-project.html>
- PostgreSQL 버전 정책과 16 문서: <https://www.postgresql.org/support/versioning/> · <https://www.postgresql.org/docs/16/>
- Yarn 보안 기능과 workspace: <https://yarnpkg.com/features/security> · <https://yarnpkg.com/features/workspaces>

## 실행 기록

| 확인 | 실제 결과 |
|---|---|
| `node --version` | v22.23.4 |
| `npm.cmd --version` | 10.9.9 |
| `corepack --version` | 0.36.0 |
| `corepack yarn --version` | 4.18.1 |
| `astro --version` (`@datagrizzly/web`) | 7.4.2 |
| Yarn linker / scripts / hardened / age gate | node-modules / false / true / 3d |
| Node 플랫폼 | win32 x64 |
| `java -version` / `javac -version` | 21.0.8 LTS |
| `kotlinc -version` | 2.0.21 |
| `psql --version` | PostgreSQL 16.10 |
| `pg_isready -h 127.0.0.1 -p 5432` | accepting connections |
| `rustc --version` / `cargo --version` | 1.92.0 |
| `python --version` | 3.11.5 |
| `docker --version` | 28.3.2 |
| `tsc --version` (`@datagrizzly/web`) | 6.0.3 |
| `astro check` | 3개 파일, 오류 0, 경고 0, 힌트 0 |
| C: 여유 | 19.86 GiB (S19 기록 직전). 검사 종료 시점 21.38 GiB |

## 다음 작업

1. S08 기존 Docker 점검을 한다. S07은 JDK 21.0.8을 재사용하고 `services/cms/gradle.properties`만 두었다.
2. 재현 설치는 `corepack yarn install --immutable`을 사용한다.
3. API와 DB는 S13·S14 전이다. Gradle Wrapper JVM 비교는 `gradlew.bat`가 생기는 S12에서 한다.
