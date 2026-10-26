# F13 PostgreSQL 공개 검색

> v5.0 기준: [[SCOPE_v5]] 적용. 주력 Kotlin·Rust·TypeScript·PostgreSQL. LLM·C++ 제외, Go는 국소 배포 도구로 필요할 때만 사용합니다. 로컬 자원은 [[02_RESOURCE_BUDGET]] 참조.
GET/api/search, UI/explore. 데이터는현재공개release의텍스트와metadata입니다. Kotlin이검색·접근상태를검사하고Rust는발행시텍스트를추출합니다.

수용조건: 제목/주제/프로젝트필터·근거문단·원문절링크, 초안/철회제외, 결과없음과오류구분,30문항평가. 후보SQL검색은한국어품질을측정하며FTS가한국어형태소를자동해결한다고가정하지않습니다.

[[M5-01]] [[M5-02]] [[M5-03]] [[M5-04]] · [[SCOPE_v5]]
