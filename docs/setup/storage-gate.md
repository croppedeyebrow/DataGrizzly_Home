# S01 저장공간·OS·WSL 진입 게이트 기록

- 실행일: 2026-10-25 01:42 KST
- 실제 프로젝트 루트: `C:\Dev\sideproject\My_Web`
- 태스크 상태: **완료 — 현재 WEB 단계 진행 허용, OS·WSL 후속 게이트 유지**
- 변경 정책: 조회와 기록만 수행; 파일 삭제·앱 제거·OS 업데이트·WSL 설치·Docker 기동 없음
- Git: 사용자 요청에 따라 계속 보류

## 게이트 결론

| 게이트 | 관측 결과 | 판정 |
|---|---|---|
| C: 여유 공간 | 25.98 GiB | 현재 DOCS/WEB 단계 진행 가능; 40 GiB는 대형 작업 전 목표 |
| 중단 하한 | 15 GiB 이상 | 즉시 중단 상태는 아니지만 대형 설치 금지 |
| Windows 지원 | Windows 11 Home 23H2, 빌드 22631.6199 | 지원 종료 버전; 운영 연결·대형 환경 확장 전 업데이트 필요 |
| WSL 버전 | 3.0.3.0, Linux 커널 6.18.54.1-1 | Docker 최소 WSL 2.1.5 충족 |
| WSL 상태·배포판 | 사용자 PowerShell 확인: `docker-desktop`만 Stopped/WSL2 | 개발용 Ubuntu 미설치; M4 전까지 설치 불필요 |
| Docker 저장공간 | Docker Desktop 4.43.2 설치, 엔진 중지 | 필요 시 시작 후 `docker system df` 재측정 |
| Docker 실행 컨테이너 | 데몬 연결 실패 | 미확인; Docker를 강제로 시작하지 않음 |

S01의 조사와 기록을 완료했다. Node, Kotlin, PostgreSQL, Rust, Docker Desktop은 이미 설치되어 있으므로 해당 도구의 신규 설치 용량을 추가로 확보할 필요가 없다. 현재 25.98 GiB에서 문서와 작은 WEB 개발을 진행하고, 매 단계 후 여유 공간이 15 GiB 아래로 내려가지 않는지 측정한다. 40 GiB는 OS 기능 업데이트, 새 Ubuntu 설치가 실제로 필요한 경우, 대형 Docker 이미지처럼 증가량이 큰 작업 전의 보수적 목표일 뿐 현재 단계의 필수 통과값이 아니다.

## 기존 도구 재사용 확인

| 도구 | 확인 결과 | 현재 결정 |
|---|---|---|
| Node/npm | v22.23.2 / 10.9.8 | 재설치하지 않음; S05 호환성 확정 후 재사용 |
| Kotlin CLI | 2.0.21 | 재설치하지 않음; 프로젝트 Kotlin 플러그인 버전은 별도 확정 |
| JDK | 21.0.8 LTS | 재설치하지 않음; S05 호환성 확인 |
| PostgreSQL | 16.10, Windows 서비스 실행 중, 5432 연결 수락 | 네이티브 설치 재사용 후보; S13에서 컨테이너 필요성만 판단 |
| Rust | 1.92.0 Windows MSVC | 재설치하지 않음; M4 Linux 검증과는 구분 |
| Docker Desktop | 4.43.2 설치, 엔진 중지 | 재설치하지 않음; 필요 시 실행 상태만 검증 |
| Ubuntu | 개발용 배포판 없음; `docker-desktop`만 등록 | M4/S16에서 실제 필요와 공간을 재검토한 뒤 설치 |

## 디스크 실측

| 시점 | 사용 | 여유 | 전체 |
|---|---:|---:|---:|
| S00 기록, 2026-10-24 | 215.98 GiB | 21.59 GiB | 237.58 GiB |
| S01 실측, 2026-10-25 | 211.59 GiB | 25.98 GiB | 237.58 GiB |

S00 이후 여유 공간이 4.39 GiB 늘었지만 이번 S01 작업에서는 파일을 삭제하지 않았다. 증가 원인을 이 작업의 성과로 귀속하지 않는다.

## 주요 점유 후보

| 항목 | 측정 크기 | 분류 | 처리 원칙 |
|---|---:|---|---|
| `%LOCALAPPDATA%\Temp` | 5.66 GiB | 임시 파일 후보 | 앱을 종료한 뒤 Windows 저장소 설정에서 사용자가 선별 |
| Desktop | 2.28 GiB | 사용자 파일 | 백업·이동·삭제 여부를 사용자가 결정 |
| Documents | 1.50 GiB | 사용자 파일 | 프로젝트·문서 보존 후 사용자가 결정 |
| Rust toolchains | 1.18 GiB | 설치 자산 | 자동 삭제 금지; M4 계획과 함께 재검토 |
| `C:\ProgramData\Package Cache` | 1.02 GiB | 설치 복구 자산 | 수동 삭제 금지 |
| Cargo registry cache | 0.48 GiB | 재생성 가능 캐시 | Rust 작업이 없을 때 사용자 승인 후 정리 후보 |
| Gradle cache | 0.27 GiB | 재생성 가능 캐시 | Kotlin 작업 전 재다운로드 비용을 고려해 사용자 승인 후 결정 |
| Downloads | 0.01 GiB | 사용자 파일 | 영향 작음 |
| npm cache | 0.00 GiB | 재생성 가능 캐시 | 정리 효과 없음 |
| Docker 사용자 폴더 | 0.00 GiB로 관측 | Docker 관리 영역 | 데몬 미실행 상태라 Docker 명령으로 재검증 필요 |
| Windows Update download | 0.00 GiB로 관측 | OS 관리 영역 | 수동 삭제 금지 |

추가로 `hiberfil.sys`는 5.54 GiB, `pagefile.sys`는 10.81 GiB, `swapfile.sys`는 0.02 GiB로 관측했다. 페이지 파일은 현재 13.84 GiB RAM 환경에서 수동 삭제·축소하지 않는다. 최대 절전 모드 해제는 기능 손실과 시스템 설정 변경이므로 이번 작업에서 수행하지 않았으며, 사용자가 장단점을 확인한 뒤 별도로 결정한다.

Temp·Cargo·Gradle의 측정 합계는 약 6.41 GiB지만 지금 이를 모두 정리할 필요는 없다. 실제 Temp에는 사용 중인 파일이 있을 수 있으므로 보장된 회수량도 아니다. 새 패키지·빌드 후 C: 여유를 다시 측정하고, 예상 증가량을 뺀 뒤에도 15 GiB 이상이면 다음 작은 단계를 진행한다.

## OS와 Docker 공식 조건

- Microsoft는 Windows 11 Home/Pro 23H2가 2025-11-11에 업데이트 종료에 도달했다고 안내한다. 현재 장치에 Windows Update로 제공되는 지원 버전으로 이동해야 한다. <https://learn.microsoft.com/en-us/lifecycle/announcements/windows-11-23h2-end-of-updates-home-pro>
- Docker Desktop은 Microsoft 서비스 기간 안의 Windows만 지원한다. WSL 백엔드는 WSL 2.1.5 이상, 8 GB RAM, SLAT, BIOS/UEFI 가상화를 요구한다. <https://docs.docker.com/desktop/setup/install/windows-install/>

현재 WSL 3.0.3.0과 RAM 13.84 GiB, 펌웨어 가상화 활성 상태는 일부 조건을 충족한다. 그러나 Windows 23H2 지원 종료, SLAT API 관측값 `False`, 배포판 조회 권한 오류 때문에 Docker/WSL 게이트는 통과로 처리하지 않는다.

## 실행 기록

| 명령 | 결과 |
|---|---|
| `Get-CimInstance Win32_OperatingSystem` | `액세스가 거부되었습니다`; PowerShell cmdlet 실패 |
| 레지스트리 OS 조회 | 23H2, 빌드 22631.6199, Core/Home |
| `Get-PSDrive C` | 사용 211.59 GiB, 여유 25.98 GiB |
| `docker system df` | 데몬 파이프 없음, 종료 코드 1 |
| `docker ps` | 데몬 파이프 없음, 종료 코드 1 |
| `wsl --version` | 성공, WSL 3.0.3.0 |
| `wsl --status` | `E_ACCESSDENIED`, 종료 코드 -1 |
| `wsl -l -v` | `E_ACCESSDENIED`, 종료 코드 -1 |
| 주요 사용자·캐시 폴더 크기 조회 | 위 표에 기록 |

측정 스크립트 작성 중 PowerShell의 `foreach (...) { ... } | ...` 구문 오류가 두 번 발생했다. 두 경우 모두 조회가 시작되기 전에 중단되어 시스템 변경은 없었다. 반복문 결과를 변수에 저장한 뒤 출력하는 방식으로 수정해 재실행했다.

## 사용자 수행 단계

1. 중요한 사용자 파일을 먼저 백업한다.
2. 현재는 용량 정리를 강제하지 않고 작은 WEB 단계 후 C: 여유를 재측정한다.
3. 여유가 15 GiB에 가까워지거나 Windows Update가 추가 공간을 요구할 때 Windows 설정 → 시스템 → 저장소에서 임시 파일과 설치된 앱을 선별한다.
4. Windows Update에서 이 장치에 제공되는 지원 Windows 11 버전으로 업데이트하고 재부팅한다.
5. Ubuntu는 현재 설치하지 않고 M4/S16 진입 시점에만 필요성과 공간을 다시 확인한다.
6. Docker를 사용할 시점에 Docker Desktop을 실행한 뒤 `docker system df`를 다시 측정한다. DB volume·VHDX·전체 prune은 자동 삭제하지 않는다.

## 다음 인계

- S01 상태: 완료
- 계속 가능한 범위: DOCS, WEB 및 기존 도구의 읽기 전용 검증
- 후속 게이트: 지원 Windows 업데이트, M4 전 Ubuntu 설치 필요성 재검토, Docker 엔진 사용량 확인
- 중단 조건: C: 여유 15 GiB 미만 또는 지속적인 메모리 압박
- 다음 문서 태스크: S02 Windows 에디터 확인; Git 관련 S03·S04는 사용자 설정 때까지 보류
