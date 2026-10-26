# 13. PostgreSQL 공개 검색

> v5.0 기준: [[SCOPE_v5]] 적용. 주력 Kotlin·Rust·TypeScript·PostgreSQL. LLM·C++ 제외, Go는 국소 배포 도구로 필요할 때만 사용합니다. 로컬 자원은 [[02_RESOURCE_BUDGET]] 참조.
Kotlin discovery모듈이PostgreSQL의공개release별검색행을조회합니다. RustCLI가본문검증/정규화/절추출을담당하고결과를발행작업에전달합니다. 별도검색서버·C++·벡터DB·모델은없습니다.

[[SCOPE_v5]]의검색스키마·입력정책을적용합니다. 원본Entry/Revision과파생검색행을분리하고releaseId·revisionId·canonical·anchor를보존합니다. 검색쿼리에서현재공개세대와철회목록을반영합니다. 초기소규모부분일치→평가→필요시PostgreSQL인덱스전략확장순서입니다.

[[M5-01]] [[M5-02]] [[M5-03]] [[M5-04]]
