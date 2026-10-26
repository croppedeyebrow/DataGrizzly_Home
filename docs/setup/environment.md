# S00 환경과 실제 사양 기록

- 실행일: 2026-10-24 22:19 KST
- 적용 문서: DataGrizzly 개발 문서 v5.0
- 태스크 상태: **완료 — 환경 진단과 실행 모드 결정 완료**
- 실제 프로젝트 루트: `C:\Dev\sideproject\My_Web`
- 기본 셸: Windows PowerShell 7.6.5
- Git 작업: **사용자 요청으로 보류** — 저장소 초기화, 작성자 설정, 커밋, 원격 연결을 수행하지 않음

## 판정

S00의 읽기 전용 환경 조사와 개발 경로 결정은 완료했다. 현재 자원 상태에서는 v5.0 자원 정책에 따라 `DOCS`와 작은 `WEB` 작업만 허용한다. 지원 OS, 저장 공간, WSL 상태는 S01 게이트에서 해결해야 한다. 이 파일의 완료 표시는 제품 구현이나 S01 통과를 뜻하지 않는다.

## 실제 시스템

| 항목 | 2026-10-24 관측값 | 판정 |
|---|---|---|
| CPU | AMD Ryzen 5 7530U with Radeon Graphics, 논리 프로세서 12개 | 작은 병렬도로 개발 가능 |
| 메모리 | 전체 13.84 GiB, 측정 순간 가용 2.05 GiB | 동시 서비스 실행 금지; 지속 사용률은 별도 관측 필요 |
| C: 드라이브 | 전체 237.58 GiB, 사용 215.98 GiB, 여유 21.59 GiB | Ubuntu·대형 SDK·전체 컨테이너 준비 금지; 40 GiB 확보 목표 |
| OS | Windows 11 Home 23H2, x64, 빌드 `22631.6199` | 지원 종료 버전; S01에서 업데이트 게이트 처리 |
| 펌웨어 가상화 | 활성화 | 확인 |
| SLAT | Windows API 관측값 `False` | WSL2 요구조건과 충돌하므로 사용자 세션에서 재확인 필요 |
| PowerShell | 7.6.5 Core | 기본 셸로 사용 |

레지스트리의 `ProductName`은 호환성 때문에 `Windows 10 Home`으로 표시되지만 `DisplayVersion=23H2`와 빌드 22631을 기준으로 Windows 11 23H2로 기록한다.

기존 PC 프로필 문서에는 C: 여유 공간이 23.4 GiB로 기록되어 있다. 이번 실측은 21.59 GiB이며, 약 1.81 GiB 감소했다. 이후 게이트 판단은 더 최근 값인 21.59 GiB를 사용한다.

## 설치 도구

| 도구 | 상태 | 관측 버전 |
|---|---|---|
| Git | 설치됨 | 2.43.0.windows.1 |
| Node.js | 설치됨 | v22.23.2 |
| npm | 설치됨 | 10.9.8 |
| Java/JDK | 설치됨 | Java/Javac 21.0.8 LTS |
| Kotlin CLI | 설치됨 | kotlinc-jvm 2.0.21 — `C:\Dev\KOTLIN\kotlinc` |
| PostgreSQL | 설치·실행 중 | 16.10, Windows 서비스 `postgresql-x64-16`, `127.0.0.1:5432` 연결 수락 |
| Docker Desktop | 설치됨·엔진 중지 | 4.43.2 |
| Docker CLI | 설치됨 | 28.3.2, build 578ccf6 |
| Rust | 설치됨 | rustc/cargo 1.92.0, Windows MSVC toolchain |
| Python | 설치됨 | Anaconda Python 3.11.5 |
| CMake | 없음 | 이번 범위의 C++ 제외 정책과 일치 |
| Go | 없음 | 실제 배포 필요가 확인될 때만 검토 |

설치되어 있다는 사실은 프로젝트 버전 확정이나 태스크 완료를 의미하지 않는다. Node·npm·JDK의 재사용 여부는 S05 공식 호환성 검토에서 확정한다. Windows Rust는 M4의 Ubuntu 기반 S16을 완료한 것으로 간주하지 않는다. Python은 S20 전까지 프로젝트 의존성으로 사용하지 않는다.

Node, Kotlin, PostgreSQL, Rust, Docker Desktop은 신규 설치 대상으로 계산하지 않는다. 프로젝트 의존성·빌드 캐시·DB 데이터 증가량만 단계별로 측정한다. PostgreSQL은 기존 네이티브 16.10을 우선 재사용 후보로 두며, S13에서 Docker DB가 반드시 필요한지 계약과 격리 요구를 검토한 뒤 결정한다.

## WSL과 Docker 관측

`wsl -l -v`는 현재 Codex 실행 세션에서 다음 오류로 실패했다.

```text
Wsl/EnumerateDistros/Service/E_ACCESSDENIED
exit code: -1
```

Codex 실행 세션에서는 배포판 조회가 `E_ACCESSDENIED`로 실패했지만, 사용자가 2026-10-25 사용자 PowerShell에서 실행한 `wsl -l -v` 결과를 제공했다. 등록된 배포판은 `docker-desktop` 하나이며 Stopped/WSL2 상태다. 개발용 Ubuntu는 설치되어 있지 않다. `docker-desktop`은 개발 셸로 사용하지 않으며 Ubuntu는 M4/S16에서 실제 필요가 생길 때만 설치를 검토한다. Docker Desktop은 설치되어 있으나 엔진은 현재 중지 상태다.

## 실행 모드 결정

| 모드 | 현재 허용 | 조건 |
|---|---|---|
| DOCS | 예 | 문서 검토와 작은 파일 작업 |
| WEB | 조건부 | Astro 또는 관리자 중 하나만 실행하고 메모리·디스크를 관측 |
| CMS | 아니요 | S01, S07~S14와 DB 준비 후 |
| PUBLISH | 아니요 | CMS와 발행 기능 구현 후 |
| SEARCH | 아니요 | PostgreSQL/Kotlin 검색 구현 후 |
| NATIVE_BUILD | 아니요 | 40 GiB 공간 목표와 S16 선행조건 통과 후 |
| OPS | 아니요 | M6 단계에서 필요 범위 확정 후 |
| WORKFLOW | 아니요 | S20~S22와 M8 단계에서 준비 후 |

메모리 측정은 한 시점의 스냅샷이다. v5.0의 `85%가 1분 이상 지속` 중단 조건 충족 여부를 이 값 하나로 단정하지 않는다.

## 실행 기록

| 명령 또는 확인 | 결과 |
|---|---|
| `Get-Command git,node,npm,java,docker` | 모두 발견 |
| `Get-PSDrive C` | 여유 21.59 GiB |
| `wsl -l -v` | 실패, `E_ACCESSDENIED`, 종료 코드 -1 |
| 레지스트리 OS 정보 | 23H2, 빌드 22631.6199, Core/Home, x64 |
| `GlobalMemoryStatusEx` | 전체 13.84 GiB, 가용 2.05 GiB |
| `IsProcessorFeaturePresent` | 펌웨어 가상화 True, SLAT False |
| 도구별 `--version` | 위 설치 도구 표에 기록 |
| `kotlinc -version` | kotlinc-jvm 2.0.21 |
| `pg_isready -h 127.0.0.1 -p 5432` | 연결 수락, PostgreSQL 16.10 서비스 실행 확인 |
| 프로젝트 `.git` 확인 | 없음; 사용자 요청에 따라 생성하지 않음 |

비밀번호, 토큰, 인증서, `.env` 원문은 조회하거나 기록하지 않았다.

## 변경 및 인계

- 경로 정규화: 문서 25개, 총 35곳을 실제 프로젝트 루트 `C:\Dev\sideproject\My_Web`로 변경
- 생성 파일: `docs/setup/environment.md`
- 수행하지 않은 작업: Git 초기화·설정·커밋·원격 연결, 패키지 설치, OS 업데이트, WSL/Ubuntu 설치, Docker 컨테이너 실행
- 다음 태스크: S01 저장공간·OS·WSL 진입 게이트
- Git 관련 S03·S04: 사용자가 Git을 설정할 때까지 보류
