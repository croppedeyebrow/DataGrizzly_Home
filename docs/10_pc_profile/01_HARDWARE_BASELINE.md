# 현재 PC 기준과 개발 가능 범위 — v5.0

기준일 2026-10-24. 사양은 사용자가 직접 보낸 PowerShell 결과입니다. 벤치마크나 실제 설치 결과는 아닙니다.

| 항목 | 확인값 | 해석 |
|---|---|---|
| CPU | AMD Ryzen 5 7530U, 6코어/12논리프로세서 | 기능 개발 대상은 유지, 컴파일 병렬은 작게 시작 |
| 메모리 | TotalPhysicalMemory 조회 결과13.8GiB | 이를16GB 또는 현재 여유 RAM으로 바꾸어 쓰지 않음 |
| GPU | AMD Radeon (TM) Graphics | VRAM 미확인, CUDA·ROCm·GPU 추론을 전제로 하지 않음 |
| C: | 총237.6GiB, 여유23.4GiB | 현재 가장 먼저 관리할 제약 |
| Windows | Windows11 Home, 10.0.22631, x64 | 23H2. 지원 중인 버전으로 업데이트 확인 필요 |
| WSL | WSL 3.0.3.0. 사용자 PowerShell 확인 결과 `docker-desktop`만 Stopped/WSL2 | 개발용 Ubuntu는 미설치; M4 전까지 불필요 |

## 결론

Astro/React·Kotlin·PostgreSQL 기반 기능 개발은 단계별 실행으로 진행하는 설계를 채택합니다. 노트북에서 모든 언어 빌드·이미지 생성·추론을 한꺼번에 실행하는 것을 요구하지 않습니다. 실제 실행 가능 판정은 단계별 측정으로 결정합니다. 처리속도·첫 빌드 시간·LLM 토큰속도를 사양만으로 보장하지 않습니다.

## 변경된 기본 경로

초기에는 Windows PowerShell의 Git·Node·Kotlin CLI·JDK를 재사용합니다. PostgreSQL 16.10 네이티브 서비스도 설치·실행 상태이므로 S13에서 격리 요구를 검토한 뒤 재사용 여부를 결정합니다. Docker Desktop과 Rust도 설치되어 있어 중복 설치하지 않습니다. 개발용 Ubuntu는 현재 없으며 M4/S16에서 실제 필요가 생길 때만 설치합니다. Go는 배포 중 구체적 필요가 확인될 때만 설치합니다. C++·모델은 설치하지 않습니다. Python 환경은 S20~S22에서 별도로 준비합니다.

주력은 Kotlin·Rust·TypeScript·PostgreSQL입니다. Rust는 콘텐츠 검증·정규화·청킹·내보내기, Kotlin은 CMS·발행·검색 API, PostgreSQL은 원본과 공개 검색 데이터를 담당합니다. 생성형 답변 UI·API는 이번 출시 범위에서 제외합니다.

## 지금 해도 되는 일과 먼저 해결할 일

문서 검토·기존 도구 조회·작은 fixture 작성은 바로 가능합니다. Node 설치/웹 의존성 추가는 설치량과 여유 공간을 확인하며 진행합니다. OS 업데이트·Ubuntu·대형 SDK·컨테이너 빌드·모델 다운로드는 디스크 계획을 통과한 뒤 수행합니다. 현재23.4GiB로 전체 개발 환경을 충분히 운영할 수 있다고 선언하지 않습니다.

Windows Home23H2는 Microsoft 공지상2025-11-11 업데이트 종료 대상입니다. OS 업데이트와 기존 Docker 지원 상태를 확인한 뒤 인증 자격증명·원격 운영 연결을 설정합니다. 실제 업데이트를 이 문서 작성 과정에서 실행한 것은 아닙니다.

[공식 자료](06_SOURCES.md) · [자원 정책](02_RESOURCE_BUDGET.md) · [진행 순서](03_EXECUTION_PLAN.md)
