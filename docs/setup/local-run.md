# 로컬 실행

- 기록일: 2026-10-25
- 확인한 범위: 공개 웹 `@datagrizzly/web` 하나
- API, 관리자, PostgreSQL, Rust는 실행하지 않았다

명령은 저장소 루트에서 실행한다. 경로는 그 루트의 상대 경로다.

## 공개 웹

탐색기에서 저장소 루트의 `dev-web.cmd`를 실행하면 개발 서버가 켜진다. PowerShell에서는 `.\dev-web.cmd`다. 주소는 http://127.0.0.1:4321/ 이고, 창을 닫거나 `Ctrl+C`로 멈춘다. `node_modules`가 없을 때만 `corepack yarn install --immutable`을 먼저 실행한다.

`apps/web/astro.config.mjs`의 개발 서버는 `127.0.0.1:4321`이다. 4321이 이미 사용 중이면 Astro는 다음 포트로 옮겨 간다. 이번 확인에서는 4321이 비어 있었고 그 포트가 응답했다.

```powershell
corepack yarn install --immutable
corepack yarn workspace @datagrizzly/web build
corepack yarn workspace @datagrizzly/web preview --host 127.0.0.1 --port 4321
node scripts/check-m0-env.mjs
```

2026-10-25 실행 결과:

| 명령 | 종료 코드 | 결과 |
|---|---:|---|
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\check.ps1` | 3 | 설치, 정적 빌드, 타입 검사, 환경 분류는 통과. Git 저장소가 없어 통과가 아님 |
| `corepack yarn workspace @datagrizzly/web preview --host 127.0.0.1 --port 4321` | 응답 후 종료 | `http://127.0.0.1:4321/` HTTP 200. 본문은 제목 DataGrizzly와 아직 API·콘텐츠를 연결하지 않았다는 문장 |
| `npm.cmd --version` | 0 | 10.9.9. `package-lock.json`은 없음. 설치는 Yarn만 사용 |

빌드 로그는 `output: "static"`, 페이지 1개, adapter 없음이다. `yarn install`은 peer dependency 경고 YN0086을 남기고 종료 코드 0이었다.

미리보기를 띄운 뒤에는 프로세스를 종료했다. 종료 후 4321 수신은 없었다.

## 포트와 설정

| 대상 | 계약에 적힌 값 | 현재 파일 | 이번 실행 |
|---|---|---|---|
| 공개 웹 | `127.0.0.1:4321` | `apps/web/astro.config.mjs` | HTTP 200 |
| 관리자 | `127.0.0.1:5173` | `apps/admin` 없음 | 미실행 |
| API | `localhost:8080` | `services/cms` 없음 | 미실행 |
| 공개 웹의 API 프록시 | 없음 | 설정 없음 | 추가하지 않음 |
| 개발 DB | `127.0.0.1:15432` | `infra/local/compose.yaml` 없음 | 미실행 |
| Spring `local` 프로필 | `application-local.yml` | 없음 | 미실행 |

PC에 PostgreSQL 16.10이 `127.0.0.1:5432`로 설치되어 있다는 S00 기록은 유지한다. 이 저장소는 그 서버에 접속하지 않았다.

## 환경 변수

`.env.example`은 이름만 구분한다. 값으로 쓸 비밀번호와 토큰은 없다.

- `BROWSER_PUBLIC`: 브라우저 번들에 들어갈 수 있는 이름. `PUBLIC_` 변수는 정의하지 않았다.
- `SERVER_ONLY`: 이후 CMS가 프로세스 환경으로 읽을 `DB_URL`, `DB_USER`, `DB_PASSWORD`. 공개 웹은 읽지 않는다.

`.gitignore`는 `.env`와 `node_modules`, `dist`를 제외한다. Git 초기화와 커밋은 하지 않았다. S03·S04는 계속 보류다.

`node scripts/check-m0-env.mjs`는 `apps/web/dist`에서 `DB_PASSWORD`와 토큰 형태를 찾고, `apps/web` 소스의 사용자 프로필 절대 경로를 찾는다. 이번 빌드 결과는 둘 다 없었다. 검사 스크립트는 임시 파일에 넣은 `DB_PASSWORD`는 검출했다.

## DB는 이후 단계

도메인 테이블 없이 공개 웹 확인은 가능하다. DB 설치와 bootstrap migration은 M2 직전 S13·S14까지 미룬다.

그때의 확인 순서:

1. S13에서 PostgreSQL 하나를 `127.0.0.1:15432`에만 둔다.
2. S14에서 연결 확인용 메타만 만든다. 업무 테이블은 M2-01이다.
3. `DB_PASSWORD`는 저장소에 넣지 않고, 서버를 실행하는 프로세스에만 주입한다.
4. 공개 웹 `dist`에 `DB_PASSWORD`가 없는지는 `node scripts/check-m0-env.mjs`로 다시 본다.

이번 단계에서는 `docker compose`를 실행하지 않았다. Compose 파일이 없기 때문이다.

## 다른 경로에서의 재빌드

기존 `node_modules`와 `dist` 없이 다른 디렉터리에서 잠금 파일만으로 설치하는 절차는 실행하지 않았다. 검사 시작 시 C: 여유는 13.47 GiB, 검사 종료 시 12.4 GiB였다. 15 GiB 아래에서는 의존성 트리를 새로 만들지 않는다. 저장소 안의 `node_modules`는 173.2 MiB로 그대로였고, 줄어든 용량은 이 저장소 크기와 맞지 않는다.

같은 작업 디렉터리의 기존 설치 위에서는 `corepack yarn install --immutable`과 정적 빌드가 통과했다.
