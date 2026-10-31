# Docker Desktop 점검

- 실행일: 2026-10-31
- 태스크: S08
- 브랜치: `chore/s08-docker-check`
- 판정: 기존 설치를 재사용한다. 엔진이 꺼져 있어 서버 버전과 이미지 용량은 이번에 읽지 못했다. 재설치하지 않았다.

## 설치

| 항목 | 결과 |
|---|---|
| Docker Desktop 파일 버전 | 4.43.2.199162 (`C:\Program Files\Docker\Docker\Docker Desktop.exe`) |
| 클라이언트 | 28.3.2, API 1.51, windows/amd64, 컨텍스트 `desktop-linux` |
| Compose | v2.38.2-desktop.1. 종료 코드 0 |
| 엔진 | 파이프 `dockerDesktopLinuxEngine`가 없어 `docker version` 서버 조회와 `docker system df`는 종료 코드 1 |
| 프로세스 | Docker Desktop 프로세스는 없었다 |
| WSL | `docker-desktop` 하나, 상태 Stopped, 버전 2. Ubuntu 배포판은 없다 |
| Windows | 10.0.22631, amd64 |

클라이언트 조회는 종료 코드 1이지만 클라이언트 정보 자체는 출력됐다. 엔진만 연결되지 않았다.

## 디스크와 메모리

C: 여유는 조회 전후 23.27 GiB다. 15 GiB 하한보다 크다. 메모리는 전체 13.84 GiB, 여유 2.64 GiB였다. 엔진을 켜면 WSL이 같이 뜨므로 이번 점검에서는 시작하지 않았다.

## DB 하나

S13에서 PostgreSQL은 하나만 둔다. 지금 네이티브 PostgreSQL 16.10이 있고, 컨테이너 DB는 시작하지 않았다. 둘을 동시에 띄우지 않는다. Kubernetes와 모델 컨테이너는 켜지 않았다. 엔진이 꺼져 있어 기존 이미지 목록은 미실행이다.

## WSL 제한

`%UserProfile%\.wslconfig`는 없었다. 아래 시작값을 새로 적었다. 기존 값은 없어 덮어쓰지 않았다.

```ini
[wsl2]
memory=4GB
processors=4
swap=1GB
```

`wsl --shutdown`은 실행하지 않았다. 이 값은 다음 WSL 시작 전에 적용 절차가 남아 있다. `docker-desktop`은 이미 Stopped라 끄고 기다릴 DB 컨테이너는 없었다.
