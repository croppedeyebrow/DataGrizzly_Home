# 전체 화면 목록과 태스크 연결

> v5.0 기준: [[SCOPE_v5]] 적용. 주력 Kotlin·Rust·TypeScript·PostgreSQL. LLM·C++ 제외, Go는 국소 배포 도구로 필요할 때만 사용합니다. 로컬 자원은 [[02_RESOURCE_BUDGET]] 참조.
27개 데스크톱 화면 템플릿입니다. 상태 변형과 콘텐츠 종류는 같은 템플릿을 재사용할 수 있습니다. 아래 관리자 보조 화면도 각 태스크에서 누락 없이 구현합니다.

| ID | 화면 | 경로 | 기능 | 태스크 |
|---|---|---|---|---|
| P01 | 홈 | `/` | F01 | [M1-01](../04_tasks/M1-01.md) |
| P02 | 소개 | `/about` | F01 | [M1-01](../04_tasks/M1-01.md) |
| P03 | 프로젝트 목록 | `/projects` | F02 | [M1-02](../04_tasks/M1-02.md) |
| P04 | 프로젝트 설명 | `/projects/{slug}` | F02 | [M1-02](../04_tasks/M1-02.md) |
| P05 | 프로젝트 과정 목록 | `/projects/{slug}/logs` | F02 | [M1-02](../04_tasks/M1-02.md) |
| P06 | 과정 기록 상세 | `/logs/{slug}` | F02 | [M1-02](../04_tasks/M1-02.md) |
| P07 | 노트 목록 | `/notes` | F03 | [M1-03](../04_tasks/M1-03.md) |
| P08 | 노트 상세 | `/notes/{slug}` | F03 | [M1-03](../04_tasks/M1-03.md) |
| P09 | 리서치 목록 | `/research` | F03 | [M1-03](../04_tasks/M1-03.md) |
| P10 | 논문 리뷰 상세 | `/research/{slug}` | F03 | [M1-03](../04_tasks/M1-03.md) |
| P11 | 뉴스 정리 상세 | `/research/{slug}` | F03 | [M1-03](../04_tasks/M1-03.md) |
| P12 | 검색과 선택적 질문 | `/explore` | F13/F14 | [M5-03](../04_tasks/M5-03.md) |
| P13 | 주제별 목록 | `/topics/{slug}` | F04 | [M1-04](../04_tasks/M1-04.md) |
| P14 | 시리즈 상세 | `/series/{slug}` | F04 | [M1-04](../04_tasks/M1-04.md) |
| P15 | 빈 결과 · 오류 · 404 | `/404 / 검색 상태` | F04 | [M1-04](../04_tasks/M1-04.md) |
| A01 | 관리자 로그인 | `/admin/login` | F05 | [M2-01](../04_tasks/M2-01.md) |
| A02 | 관리 홈 | `/admin` | F05 | [M2-01](../04_tasks/M2-01.md) |
| A03 | 게시물 관리 | `/admin/entries` | F06 | [M2-02](../04_tasks/M2-02.md) |
| A04 | 블록 편집기 | `/admin/entries/{id}/edit` | F06 | [M2-02](../04_tasks/M2-02.md) |
| A05 | 미리보기 | `/admin/entries/{id}/preview` | F08 | [M2-04](../04_tasks/M2-04.md) |
| A06 | 수정 이력과 복원 | `/admin/entries/{id}/revisions` | F08 | [M2-04](../04_tasks/M2-04.md) |
| A07 | 프로젝트 관리 | `/admin/projects` | F02 | [M2-03](../04_tasks/M2-03.md) |
| A08 | 분류와 시리즈 관리 | `/admin/taxonomies` | F04 | [M2-03](../04_tasks/M2-03.md) |
| A09 | 미디어 관리 | `/admin/media` | F07 | [M2-03](../04_tasks/M2-03.md) |
| A10 | 발행 작업 | `/admin/publications` | F09/F10 | [M3-01](../04_tasks/M3-01.md) |
| A11 | 검색·모델 상태 | `/admin/search` | F13/F14 | [M5-04](../04_tasks/M5-04.md) |
| A12 | 사이트 설정 | `/admin/settings` | F11/F15 | [M6-04](../04_tasks/M6-04.md) |

개별 화면 명세는 [이미지 모음 안내](../08_ui_assets/README.md)에서 접근합니다. F14 생성은 선택이며 M7 태스크를 추가 적용합니다. A02는 M2~M5 상태를 단계적으로 연결하고 A11의 모델 상태는 M7에서 확장합니다. A12는 M6-04에서 저장 API·권한·validation까지 구현하고 프론트 폼만으로 완료하지 않습니다.

## v5.0 자료 워크플로 반영

관리자 화면 A13 자료 등록, A14 작업 목록·상세, A15 검토·초안 생성 추가. 현재 총 30개 화면 명세. [[A13]] [[A14]] [[A15]]
