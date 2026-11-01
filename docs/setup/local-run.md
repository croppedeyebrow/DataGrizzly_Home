# 로컬 실행

- 기록일: 2026-10-25. CMS health 확인은 2026-10-31
- 확인한 범위: 2026-10-25에는 공개 웹 `@datagrizzly/web` 하나
- 2026-10-25 확인에서는 API, 관리자, PostgreSQL, Rust를 실행하지 않았다

명령은 저장소 루트에서 실행한다. 경로는 그 루트의 상대 경로다.

## 공개 웹

탐색기에서 저장소 루트의 `dev-web.cmd`를 실행하면 개발 서버가 켜진다. PowerShell에서는 `.\dev-web.cmd`다. 주소는 http://127.0.0.1:4321/ 이고, 창을 닫거나 `Ctrl+C`로 멈춘다. `node_modules`가 없을 때만 `corepack yarn install --immutable`을 먼저 실행한다.

`apps/web/vite.config.ts`의 개발 서버는 `127.0.0.1:4321`이다. 포트가 이미 사용 중이면 서버는 다른 포트로 옮겨 가지 않는다.

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

2026-10-25 빌드 로그는 당시 Astro의 `output: "static"`이었다. 2026-11-01 이후 공개 웹은 Vite가 React 페이지를 정적 HTML로 뽑는다. `yarn install`은 peer dependency 경고 YN0086을 남기고 종료 코드 0이었다.

미리보기를 띄운 뒤에는 프로세스를 종료했다. 종료 후 4321 수신은 없었다.

## 포트와 설정

| 대상 | 계약에 적힌 값 | 현재 파일 | 이번 실행 |
|---|---|---|---|
| 공개 웹 | `127.0.0.1:4321` | `apps/web/vite.config.ts` | React 정적 빌드 |
| 관리자 | `127.0.0.1:5173` | `apps/admin` | 2026-10-25에는 미실행. S11에서 자리표시자만 확인 |
| API | `127.0.0.1:8080` | `services/api` | Rust health. `/actuator/env`는 404 |
| 공개 웹의 API 프록시 | 없음 | 설정 없음 | 추가하지 않음 |
| 개발 DB | `127.0.0.1:15432` | `infra/local/compose.yaml` | 2026-11-01 컨테이너 기동. CMS는 아직 접속하지 않음 |
| 이전 Kotlin 프로필 | `services/cms` | 확장하지 않음 | 현재 health는 Rust |

PC에 PostgreSQL 16.10이 `127.0.0.1:5432`로 설치되어 있다는 S00 기록은 유지한다. 이 저장소는 그 서버에 접속하지 않았다.

## CMS health

2026-10-31에 `services/cms`에서 Gradle Wrapper 9.7.0으로 테스트와 bootJar를 만들었다. 앱은 Gradle을 끈 뒤 따로 실행했다.

```powershell
Set-Location C:\Dev\sideproject\My_Web\services\cms
.\gradlew.bat test --max-workers=2
.\gradlew.bat bootJar --max-workers=2
.\gradlew.bat --stop
java -Xms128m -Xmx768m -jar build\libs\cms-0.0.1-SNAPSHOT.jar --spring.profiles.active=local
```

`http://127.0.0.1:8080/actuator/health`는 `status=UP`이었다. `/actuator/env`는 404였다. 확인 후 Java 프로세스를 종료했고 8080 수신은 없었다. 데이터베이스와 콘텐츠 API는 이 명령에 없다. 공개 웹이나 관리자와 같이 켜지 않는다.

2026-11-01부터 같은 health는 `services/api`의 Rust 서버가 맡는다. `services/cms`는 확장하지 않는다.

```powershell
Set-Location C:\Dev\sideproject\My_Web\services\api
cargo test -j 2
cargo run
```

## 환경 변수

`.env.example`은 이름만 구분한다. 값으로 쓸 비밀번호와 토큰은 없다.

- `BROWSER_PUBLIC`: 브라우저 번들에 들어갈 수 있는 이름. `PUBLIC_` 변수는 정의하지 않았다.
- `SERVER_ONLY`: 이후 CMS가 프로세스 환경으로 읽을 `DB_URL`, `DB_USER`, `DB_PASSWORD`. 공개 웹은 읽지 않는다.

`.gitignore`는 `.env`와 `node_modules`, `dist`를 제외한다. 2026-10-26 다른 경로 빌드에서는 커밋하지 않았다.

`node scripts/check-m0-env.mjs`는 `apps/web/dist`에서 `DB_PASSWORD`와 토큰 형태를 찾고, `apps/web` 소스의 사용자 프로필 절대 경로를 찾는다. 이번 빌드 결과는 둘 다 없었다. 검사 스크립트는 임시 파일에 넣은 `DB_PASSWORD`는 검출했다.

## DB는 이후 단계

도메인 테이블 없이 공개 웹 확인은 가능하다. DB 설치와 bootstrap migration은 M2 직전 S13·S14까지 미룬다.

그때의 확인 순서:

1. S13에서 PostgreSQL 하나를 `127.0.0.1:15432`에만 둔다.
2. S14에서 연결 확인용 메타만 만든다. 업무 테이블은 M2-01이다.
3. `DB_PASSWORD`는 저장소에 넣지 않고, 서버를 실행하는 프로세스에만 주입한다.
4. 공개 웹 `dist`에 `DB_PASSWORD`가 없는지는 `node scripts/check-m0-env.mjs`로 다시 본다.

2026-10-25 확인에서는 `docker compose`를 실행하지 않았다. 그때는 Compose 파일이 없었다.

## 개발 DB

2026-11-01에 `infra/local/compose.yaml`로 PostgreSQL 16.10 컨테이너 하나만 띄웠다. 주소는 `127.0.0.1:15432`이고 데이터는 named volume `datagrizzly_pg`에 있다. 네이티브 `postgresql-x64-16`은 중지된 상태였고 켜지 않았다.

```powershell
Set-Location C:\Dev\sideproject\My_Web
docker compose --env-file .env -f infra/local/compose.yaml up -d db
docker compose --env-file .env -f infra/local/compose.yaml exec db pg_isready -U datagrizzly -d datagrizzly
```

`pg_isready`는 종료 코드 0이었다. `stop` 후 `start`에도 확인용 행이 남았고, 그 테이블은 지웠다. CMS는 이 데이터베이스에 접속하지 않았다. 비밀번호는 추적하지 않는 `.env`에만 있다. 근거는 `docs/evidence/S13.md`다.

## 다른 경로에서의 재빌드

2026-10-26에 원본 `C:\Dev\sideproject\My_Web`의 `node_modules`는 남기고, `C:\Dev\sideproject\_m003_clean`으로 소스와 `yarn.lock`만 복사했다. 복사에서 `node_modules`, `dist`, `.astro`, `.git`은 빠졌다. 복사 직후 그 폴더에는 `node_modules`와 `dist`가 없었고 `yarn.lock`은 있었다. C: 여유는 시작 20.64 GiB, 설치 후 20.42 GiB였다.

```powershell
robocopy C:\Dev\sideproject\My_Web C:\Dev\sideproject\_m003_clean /E /XD node_modules dist .astro .git
Set-Location C:\Dev\sideproject\_m003_clean
corepack yarn install --immutable
corepack yarn workspace @datagrizzly/web build
node .\scripts\check-m0-env.mjs
corepack yarn workspace @datagrizzly/web preview --host 127.0.0.1 --port 4321
```

| 명령 | 종료 코드 | 결과 |
|---|---:|---|
| `robocopy` | 1 | 파일 복사 완료. 이 코드는 복사 성공이다 |
| `corepack yarn install --immutable` | 0 | peer dependency 경고 YN0086 |
| `corepack yarn workspace @datagrizzly/web build` | 0 | `output: "static"`, 페이지 1개. 출력 경로는 `_m003_clean\apps\web\dist` |
| `node .\scripts\check-m0-env.mjs` | 0 | `DB_PASSWORD`와 사용자 프로필 절대 경로 없음 |
| preview `http://127.0.0.1:4321/` | HTTP 200 | 제목 DataGrizzly와 골격 문장. 확인 후 프로세스를 종료했고 4321 수신은 없었다 |

`npm.cmd --version`은 10.9.9이고 `package-lock.json`은 없었다. 설치는 Yarn만 사용했다. 확인이 끝난 뒤 `_m003_clean`은 삭제했다. 삭제 후 C: 여유는 20.63 GiB이고, 원본 `node_modules`는 그대로다. API와 DB는 실행하지 않았다.
