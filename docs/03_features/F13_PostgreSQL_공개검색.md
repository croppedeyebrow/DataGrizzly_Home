# F13 PostgreSQL 공개 검색

> v5.0 기준: [[SCOPE_v5]] 적용. 주력 React·TypeScript·Tailwind·Rust·PostgreSQL. LLM·C++ 제외, Go는 국소 배포 도구로 필요할 때만 사용합니다. 로컬 자원은 [[02_RESOURCE_BUDGET]] 참조.
GET /api/search, UI /explore. 데이터는 현재 공개 release의 텍스트와 metadata입니다. Rust API가 검색과 접근 상태를 검사하고, 발행 시 텍스트도 추출합니다. M5-01에서 검색 후보 표와 30문항 fixture를 넣었다. M5-02에서 검색 조건은 활성 릴리스, 공개 수정본, 철회 목록만 본다. M5-03에서 `GET /api/search`와 `/explore`를 연결했다. `/api/ask`는 만들지 않고 탐색으로 안내한다. 실제 DB 검색 행은 아직 조회하지 않았다.

수용조건: 제목/주제/프로젝트필터·근거문단·원문절링크, 초안/철회제외, 결과없음과오류구분,30문항평가. 후보SQL검색은한국어품질을측정하며FTS가한국어형태소를자동해결한다고가정하지않습니다.

[[M5-01]] [[M5-02]] [[M5-03]] [[M5-04]] · [[SCOPE_v5]]
