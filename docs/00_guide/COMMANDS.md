# v5.0 명령과 플랫폼 경계

[[04_WINDOWS_COMMANDS]]가 현재 PC의 실행 기준입니다. Windows 웹 작업은 Corepack으로 고정한 Yarn을, API는 Cargo를 사용합니다. 초기 경로는 `C:\Dev\sideproject\My_Web`이며 로컬 API는 Windows Rust로 빌드합니다. Ubuntu는 로컬 API의 선행 조건이 아닙니다.

| 구성 | 실행위치 | 검증 |
|---|---|---|
| 공개 웹 | Windows 루트 | corepack yarn workspace @datagrizzly/web build |
| 관리자 | Windows 루트, S11 이후 | corepack yarn workspace @datagrizzly/admin build |
| API | Windows services/api | cargo test -j 2 |
| DB | Windows 루트 | docker compose --env-file .env -f infra/local/compose.yaml ps |
| Rust 콘텐츠 도구 | 필요 시 Linux 검증 | cargo test --workspace -j 2 |
| Go | 조건부도입후tools/dgops | go test -p 2 ./... |

공개 웹 개발 서버는 저장소 루트의 `dev-web.cmd`로 켠다. 한 번에 한 빌드, 테스트 worker 1로 시작합니다. Linux CI에서도 Corepack으로 같은 Yarn 버전을 사용하며 Windows 명령과 섞지 않습니다. CMake/C++/llama.cpp 명령은 이번 범위에서 사용하지 않습니다.

## v5.0 자료 워크플로 반영

Python 명령은 [[S20]]과 [[S22]]에 있습니다. 명령 실행 전 pyproject/CLI가 실제로 생성되었는지 확인합니다.
