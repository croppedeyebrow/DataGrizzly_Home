# v5.0 초기 설정 예시

[[04_WINDOWS_COMMANDS]]의 Rust·DB·WSL 예시를 사용합니다. 기존CMake·LinuxNode전체설치·Go초기설치예시는폐기합니다.

## 처음의package.json

```json
{"name":"datagrizzly","private":true,"workspaces":["apps/web"]}
```

앱생성후루트lockfile하나로관리하고S11에서apps/admin,공유패키지생성시packages/*를추가합니다.

## 개발DB Compose 예시

S05에서선택한PostgreSQL이미지와메이저별공식volume경로를확인한후작성합니다. 아래값은전체파일이아닌db서비스에병합할항목입니다.

```yaml
ports:
  - "127.0.0.1:15432:5432"
mem_limit: 768m
cpus: 1.0
command: ["postgres", "-c", "shared_buffers=128MB", "-c", "max_connections=20"]
```

DB_USER/PASSWORD는서버전용이며VITE_/PUBLIC_변수에넣지않습니다. .env는비공개로보존하고 .env.example에는실제비밀값을넣지않습니다.

## Rust골격 — M4직전

콘텐츠 검증 라이브러리는 `services/api`와 같은 Cargo workspace에 둔다. 로컬 API는 Windows Rust로 빌드한다. Ubuntu는 로컬 API의 선행 조건이 아니다. WASM은 선택 후속이다. TypeScript fixture를 공유하고, Linux 바이너리가 필요하면 WSL 또는 CI에서 실행한다. 같은 프로젝트의 Node 환경을 Linux에 중복 설치하지 않는다.
