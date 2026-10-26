# 프로젝트 구성 — 주력 Kotlin·Rust·TypeScript·PostgreSQL

> v5.0 기준: [[SCOPE_v5]] 적용. 주력 Kotlin·Rust·TypeScript·PostgreSQL. LLM·C++ 제외, Go는 국소 배포 도구로 필요할 때만 사용합니다. 로컬 자원은 [[02_RESOURCE_BUDGET]] 참조.
| 영역 | 기술 | 책임 |
|---|---|---|
| 공개 페이지 | Astro·TypeScript | 정적본문·SEO·프로젝트/노트/리서치 |
| 관리자·검색UI | React·TypeScript | 편집·상태·검색 |
| API | Kotlin/Spring Boot | 인증·수정본·발행·검색·권한 |
| 콘텐츠 도구 | Rust | 검증·절추출·정규화·export |
| 데이터 | PostgreSQL | 원본·관계·작업·공개검색행 |
| 배포 | Actions·Compose·스크립트 | 검사·승격·복귀·백업 |
| 조건부 보조 | Go | 구체적필요가있는국소배포CLI |

C++·생성형 LLM은 제외합니다. Python 워커는 M8에서 추가합니다. UI구조와직접포스팅기능은유지합니다. 검색은모델없이근거문단과원문절을반환합니다.

현재PC기준실행은[[03_EXECUTION_PLAN]]을따릅니다. 운영서버와개발노트북을구분하고전체서비스를노트북에상시실행하지않습니다.

## Python 구성

LangChain loader/Retriever 어댑터와 LangGraph 상태 제어를 담당하는 workers/document-workflow를 추가합니다. [[WF02_ARCHITECTURE]]
