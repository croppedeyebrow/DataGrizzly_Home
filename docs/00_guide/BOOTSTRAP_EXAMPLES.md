# v5.0 초기 설정 예시

[[04_WINDOWS_COMMANDS]]의Gradle·DB·WSL예시를사용합니다. 기존CMake·LinuxNode전체설치·Go초기설치예시는폐기합니다.

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

S16게이트통과후Ubuntu에서content-core라이브러리/content-cli실행파일만Cargo workspace로구성합니다. WASMtarget은선택후속입니다. Kotlin과TypeScript fixture를공유하고Linux바이너리는WSL호출또는CI로실행합니다. 동일프로젝트의Node/JDK환경을Linux에중복설치하지않습니다.
