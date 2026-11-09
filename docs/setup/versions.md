# S05 단계별 버전 매트릭스

- 확정일: 2026-10-25 (Asia/Seoul)
- 프로젝트 루트: `C:\Dev\sideproject\My_Web`
- 상태: **M0-04 예시 콘텐츠 fixture 통과. 공개 페이지에는 연결하지 않음. API와 DB는 미실행**
- 원칙: 기존 설치를 우선 재사용하고, 프로젝트 의존성은 yarn.lock과 Cargo.lock으로 고정한다.
- Git: 사용자 요청에 따라 초기화·설정·커밋·원격 연결을 수행하지 않음

## 결정 요약

### 공개 웹과 관리자

| 구성 | 선택 버전 | 현재 상태 | 결정 |
|---|---:|---|---|
| Node.js | **22.23.4 LTS** | 22.23.4 설치·검증 완료 | 재사용; Windows x64 공식 MSI 설치 |
| npm | **10.9.9** | 설치·검증 완료 | Node 포함 도구로 유지; 프로젝트 패키지 관리에는 사용하지 않음 |
| Corepack | **0.36.0** | 설치·검증 완료 | 프로젝트에 고정한 Yarn 실행기 |
| Yarn | **4.18.1** | S09 실행·검증 완료 | 보안 우선 프로젝트 패키지 관리자 |
| TypeScript | **6.0.3** | `apps/web`, `apps/admin` | 두 workspace에서 통일 |
| React / React DOM | **19.3.1** | `apps/web`, `apps/admin` | 공개 웹과 관리자 |
| Vite | **8.3.5** | `apps/web`, `apps/admin` | 개발 서버와 빌드 |
| `@vitejs/plugin-react` | **6.1.2** | `apps/web`, `apps/admin` | React 플러그인 |
| Tailwind CSS | **4.3.3** | `apps/web` | 공개 웹. 관리자는 아직 없음 |
| `@tailwindcss/vite` | **4.3.3** | `apps/web` | Tailwind Vite 플러그인 |

S06에서 Node 22.23.2를 동일 LTS 계열의 22.23.4로 패치 업데이트했다. 함께 설치된 npm 10.9.9는 유지하지만 프로젝트 패키지 관리는 Yarn 4.18.1로 통일한다. Yarn은 `node_modules` 호환 방식, 제3자 설치 스크립트 기본 차단, Hardened Mode, 3일 age gate를 사용한다.

TypeScript 6.0.3은 공개 웹과 관리자에 고정한 버전이다. 공개 웹 빌드는 Vite가 React 페이지를 정적 HTML로 뽑는다.

### Rust API와 데이터베이스

| 구성 | 선택 버전 | 현재 상태 | 결정 |
|---|---:|---|---|
| Rust / Cargo | **1.92.0** | Windows MSVC. `services/api` `cargo test` | 로컬 API. Ubuntu를 기다리지 않음 |
| Axum | **0.8** | `services/api/Cargo.toml` | health. 업무 API는 아직 없음 |
| SQLx | **0.8.6** | `services/api` | 15432 접속과 `schema_meta` |
| Tokio | **1** | `services/api` | 런타임 |
| PostgreSQL | **16.x** | 네이티브 16.10은 설치됨. S13에서 서비스는 중지. Compose `postgres:16.10`이 `127.0.0.1:15432` | 개발 DB는 Compose 하나. 네이티브 5432와 같이 켜지 않음 |
| PostgreSQL 유지보수 목표 | **16.15** | 16.10 | S13 전 백업·릴리스 노트 확인 후 사용자 승인 범위에서 패치 업데이트 검토 |

PostgreSQL 16은 현재 지원되는 메이저 버전이다. 설치된 16.10은 개발 접속을 수락하지만 최신 16.x 유지보수 릴리스는 16.15다. 데이터 디렉터리를 보존하고 백업 없이 설치 프로그램을 덮어쓰지 않는다.

### 후속·조건부 기술

| 구성 | 현재 버전 | S05 결정 |
|---|---:|---|
| Rust / Cargo | 1.92.0 Windows MSVC | 로컬 API에 사용. Linux 검증은 운영 전에 따로 한다 |
| Python | Anaconda Python 3.11.5 | 설치 상태만 기록; S20에서 별도 환경과 프로젝트 버전 결정 |
| Docker Desktop / CLI | 4.43.2 / 28.3.2 | 재설치하지 않음; S08에서 엔진·Compose 검증 |
| Go | 미설치 | M6에서 반복되는 배포 도구 필요가 입증될 때만 도입 |
| C++ / CMake | CMake 미설치 | 범위 제외 |
| LLM·임베딩·벡터DB | 없음 | 범위 제외 |
| Ubuntu | 미설치, WSL에는 `docker-desktop`만 존재 | M4/S16에서 Linux Rust 검증이 필요할 때만 설치 검토 |

## 공식 호환 근거

- Node.js 릴리스: Node 22와 24는 LTS이며 22.23.4가 현재 22.x 패치다. <https://nodejs.org/en/about/previous-releases>
- TypeScript 6.0.3 릴리스: <https://github.com/microsoft/TypeScript/releases>
- React 19.3: <https://react.dev/versions>
- Vite 8.3.5: <https://www.npmjs.com/package/vite>
- Tailwind CSS 4: <https://tailwindcss.com/docs>
- Rust 1.92: <https://rust-lang.org/tools/install/>
- Axum: <https://docs.rs/axum/>
- PostgreSQL 버전 정책과 16 문서: <https://www.postgresql.org/support/versioning/> · <https://www.postgresql.org/docs/16/>
- Yarn 보안 기능과 workspace: <https://yarnpkg.com/features/security> · <https://yarnpkg.com/features/workspaces>

## 실행 기록

| 확인 | 실제 결과 |
|---|---|
| `node --version` | v22.23.4 |
| `npm.cmd --version` | 10.9.9 |
| `corepack --version` | 0.36.0 |
| `corepack yarn --version` | 4.18.1 |
| Yarn linker / scripts / hardened / age gate | node-modules / false / true / 3d |
| Node 플랫폼 | win32 x64 |
| `services/api` `cargo test -j 2` | 종료 코드 0. health, env 404, URL 검사. DB 테스트는 별도 ignore 실행에서 통과 |
| `psql --version` | PostgreSQL 16.10 |
| `pg_isready -h 127.0.0.1 -p 5432` | S13 시점 응답 없음. 서비스 Stopped |
| Compose `postgres:16.10` `127.0.0.1:15432` | accepting connections. `schema_meta` 버전 1 |
| `rustc --version` / `cargo --version` | 1.92.0 |
| `python --version` | 3.11.5 |
| `docker --version` | 28.3.2 |
| `yarn workspace @datagrizzly/web check` | 종료 코드 0 |
| C: 여유 | 19.86 GiB (S19 기록 직전). 검사 종료 시점 21.38 GiB |

## 다음 작업

1. M2-02에서 블록 편집과 초안 저장을 확인했다. 다음 작업은 M2-03 첨부와 분류다.
2. 재현 설치는 `corepack yarn install --immutable`을 사용한다.
3. 콘텐츠 API는 S18 전이다. 네이티브 5432와 compose DB를 같이 켜지 않는다.
