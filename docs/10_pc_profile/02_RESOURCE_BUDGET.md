# 자원 예산·측정·중단 조건

아래 숫자는 제조사 최소 사양이 아니라 이 프로젝트의 **보수적인 시작 설정과 작업 정책**입니다. 실제 RSS/작업관리자 메모리·디스크 증가량으로 조정합니다.

## 디스크 게이트

| 상태 | 작업 정책 |
|---|---|
| 최신 실측25.98GiB | 조사·문서·기존 도구 재사용·작은 프론트 단계 진행. 일괄 설치 금지 |
| 대형 설치 전 | 업데이트 임시 공간과 예상 설치/압축해제/캐시 증가량을 먼저 계산 |
| Ubuntu 또는 전체 컨테이너 준비 전 | C: 여유40GiB 이상을 작업 목표로 확보하거나 검증된 별도 드라이브 사용. OS 업데이트 요구가 더 크면 그 요구 우선 |
| 여유15GiB 미만 | 신규 패키지/이미지/빌드/모델 작업 중단, 저장·정리·백업만 수행 |
| 새 작업 허용 조건 | 예상 최대 증가량을 뺀 뒤에도15GiB 여유를 유지. 예상이 불명확하면 하나씩 설치하고 측정 |

40GiB는 절대 최소 또는 충분조건이 아닙니다. 초기와 완료 후 모두 Windows C:를 측정합니다. WSL 내부 df의 큰 가상 여유를 실제 SSD 여유로 해석하지 않습니다. 다른 물리/외장 드라이브 존재는 확인되지 않았습니다. 구입을 필수 조건으로 두지 않습니다.

Node·Kotlin·JDK·PostgreSQL·Rust·Docker Desktop은 이미 설치되어 있으므로 해당 프로그램의 신규 설치 공간을 다시 확보하지 않습니다. 40GiB 목표는 OS 기능 업데이트, 새 Ubuntu가 실제로 필요할 때, 대형 이미지처럼 증가량이 큰 작업에만 적용합니다. 현재 WEB 단계는 15GiB 하한을 지키며 진행합니다.

## 실행 모드

| 모드 | 실행 | 정지할 대상 |
|---|---|---|
| DOCS | 에디터·브라우저/Obsidian | Docker가 불필요하면 종료, 서버·모델 없음 |
| WEB | Astro 또는 관리자 dev 중1개, fixture | API·DB·다른 dev server·모델 |
| CMS | 관리자 dev + Kotlin API + PostgreSQL1개 | 공개웹 watcher·C++·모델 |
| PUBLISH | DB·API + 정적 빌드1개 | 두 프론트 watcher·다른 빌드·모델 |
| SEARCH | DB·API + 필요한 UI1개 | 모델·전체 재빌드 |
| NATIVE_BUILD | Rust 하나만 빌드 | API·DB·프론트 watcher |
| OPS | 배포 검사 스크립트 한 작업 | 로컬 전체 재빌드 |

## 시작 설정

- Gradle heap 1024MiB, worker2, parallel=false. Kotlin compiler는 in-process로 먼저 측정합니다. 전체 RSS는 heap보다 큽니다.
- JVM 앱은 Xms128m/Xmx768m 시작. 부족하면 타 서비스 중지 후1024m 재측정; JVM 전체 메모리 제한이 아님.
- PostgreSQL 개발 컨테이너 mem_limit768m, cpus1.0, shared_buffers128MB, 연결20. 실제 Compose 설정과 docker stats로 확인합니다. 이는 단일 개발자 fixture용입니다.
- DB connection pool maximum4, minimumIdle1 제안. 초기 테스트와 동시 요청에 맞춰 조정합니다.
- WSL VM memory4GB/processors4/swap1GB 시작. 기존 .wslconfig를 보존·병합하고 Ubuntu·Docker가 공유할 수 있는 범위를 확인합니다. 전체 Windows 앱에 적용되는 상한이 아닙니다.
- Cargo -j2, 테스트 병렬1. Go를 추후 승인하여 추가하면 -p2로 시작합니다. 자원이 부족하면1로 낮춥니다.
- Node는 첫 단계에 상한을 강제하지 않습니다. heap 오류가 나면 watcher를 끄고 단독 빌드, 프로파일링 후 필요한 값만 조정합니다.

## 관측 게이트

Windows 전체 사용 메모리가 약85%를 넘는 상태가1분 이상 지속되거나 입력 지연·지속적 paging·OOM이 발생하면 동시 프로세스를 줄입니다. 85%/1분은 운영 정책 제안입니다. 메모리 여유가 회복되기 전 다음 빌드를 시작하지 않습니다. CPU 사용률100% 자체는 실패가 아니지만 장시간 체감 정지·발열에 의한 성능 저하는 반복 조건을 기록합니다.

앱·DB·WSL의 제한값을 더해 전체 PC의 정확한 사용량이라고 계산하지 않습니다. Windows·브라우저·에디터·GPU 공유메모리·파일 캐시를 함께 관측합니다.

## v5.0 자료 워크플로 반영

WORKFLOW 모드와 Python 관측 예산은 [[WF06_OPERATIONS]]를 추가 적용합니다. 모델 설치 제한은 유지하며 Python 설치는 S20에서 필요 시 수행합니다.
