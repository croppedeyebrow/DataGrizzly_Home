# PC 검토 결과 — v4 기반, v5 확장 적용

## 주요 변경

초기Ubuntu/다중언어일괄설치→Windows최소설치와단계별확장. S00~S19전부수행→필요시점별실행. 검색별도C++서비스→PostgreSQL공개검색행+Kotlin. Rust본문처리유지. LLM은 제외하며 Python 워크플로는 v5에서 추가. Go는배포국소도구로조건부. DBbootstrap와업무schema분리유지. 초기검사와운영CI분리유지.

## 검토 범위

소개·기획·기능·아키텍처·초기설정20개·개발32개·화면명세27개·다이어그램·목차의문서내용을검색하고영향항목을수정했습니다. 예전PNG64개는레이아웃참고로보존하며질문패널과C++배지는폐기표현으로명시했습니다. 이전PDF를새아키텍처문서처럼사용하지않습니다.

## 보존한 기능

웹CMS·자동저장충돌·미리보기·수정본·발행멱등성·철회·SEO·프로젝트설명/과정·학습/리서치·백업복구는유지합니다. 자원절약을이유로권한/데이터보존검증을생략하지않습니다.

## 남은 검증

PC실제벤치마크·설치상태·Docker버전·OS업데이트·디스크정리·배포비용은미검증입니다. 사용자PC설정을이번작업에서변경하지않았습니다. 정책값은시작점이며측정후조정합니다.

## 변경 파일 목록

- START_HERE.md
- README.md
- 09_screen_specs/P15.md
- 09_screen_specs/P02.md
- 09_screen_specs/P13.md
- 09_screen_specs/P10.md
- 09_screen_specs/P12.md
- 09_screen_specs/A10.md
- 09_screen_specs/P14.md
- 09_screen_specs/A11.md
- 09_screen_specs/A07.md
- 09_screen_specs/P09.md
- 09_screen_specs/P04.md
- 09_screen_specs/A01.md
- 09_screen_specs/P06.md
- 09_screen_specs/A04.md
- 09_screen_specs/P01.md
- 09_screen_specs/P07.md
- 09_screen_specs/A06.md
- 09_screen_specs/A02.md
- 09_screen_specs/A05.md
- 09_screen_specs/A03.md
- 09_screen_specs/P08.md
- 09_screen_specs/P03.md
- 09_screen_specs/A08.md
- 09_screen_specs/A09.md
- 09_screen_specs/P11.md
- 09_screen_specs/A12.md
- 09_screen_specs/P05.md
- 01_product/00_DataGrizzly.md
- 01_product/04_관리자_CMS_기능.md
- 01_product/05_저장과_공개의_상태_모델.md
- 01_product/02_콘텐츠와_프로젝트_구조.md
- 01_product/01_한_장_요약.md
- 01_product/03_사이트맵과_방문_흐름.md
- 01_product/06_SEO_AI_편집_정책.md
- 04_tasks/M1-04.md
- 04_tasks/00_00_INDEX_04_tasks.md
- 04_tasks/M5-03.md
- 04_tasks/M0-03.md
- 04_tasks/M1-02.md
- 04_tasks/M4-02.md
- 04_tasks/M3-03.md
- 04_tasks/M2-04.md
- 04_tasks/M6-01.md
- 04_tasks/M4-03.md
- 04_tasks/M7-04.md
- 04_tasks/M6-04.md
- 04_tasks/M3-04.md
- 04_tasks/M2-03.md
- 04_tasks/M7-02.md
- 04_tasks/M3-01.md
- 04_tasks/M1-01.md
- 04_tasks/M1-03.md
- 04_tasks/M2-01.md
- 04_tasks/M0-04.md
- 04_tasks/M0-02.md
- 04_tasks/M2-02.md
- 04_tasks/M5-02.md
- 04_tasks/M3-02.md
- 04_tasks/M4-04.md
- 04_tasks/M6-03.md
- 04_tasks/M7-03.md
- 04_tasks/M7-01.md
- 04_tasks/M5-04.md
- 04_tasks/M5-01.md
- 04_tasks/M0-01.md
- 04_tasks/M6-02.md
- 04_tasks/M4-01.md
- 05_diagrams/06_infrastructure.md
- 05_diagrams/04_search_rag.md
- 05_diagrams/07_cicd.md
- 05_diagrams/02_components.md
- 08_ui_assets/README.md
- 03_features/F01_홈_소개_공통탐색.md
- 03_features/00_00_INDEX_03_features.md
- 03_features/F04_분류_시리즈_탐색.md
- 03_features/F05_관리자_인증.md
- 03_features/F02_프로젝트_설명과_과정기록.md
- 03_features/F03_학습노트_논문_뉴스.md
- 03_features/F11_SEO_외부AI노출.md
- 03_features/F07_미디어_첨부관리.md
- 03_features/F12_Rust_검증_청킹_내보내기.md
- 03_features/F10_철회_예약_세대관리.md
- 03_features/F09_발행_정적빌드.md
- 03_features/F06_블록편집_자동저장.md
- 03_features/F08_미리보기_수정이력_복원.md
- 07_introduction/04_SCREEN_CATALOG.md
- 07_introduction/05_AI_DESIGN_HANDOFF.md
- 07_introduction/03_UI_UX_GUIDE.md
- 07_introduction/01_PROJECT_INTRO.md
- 07_introduction/02_PROJECT_COMPOSITION.md
- 06_bootstrap/S08.md
- 06_bootstrap/S12.md
- 06_bootstrap/S01.md
- 06_bootstrap/S00.md
- 06_bootstrap/S05.md
- 06_bootstrap/S11.md
- 06_bootstrap/S10.md
- 06_bootstrap/S04.md
- 06_bootstrap/S17.md
- 06_bootstrap/S03.md
- 06_bootstrap/S13.md
- 06_bootstrap/S19.md
- 06_bootstrap/S07.md
- 06_bootstrap/S16.md
- 06_bootstrap/S09.md
- 06_bootstrap/S06.md
- 06_bootstrap/S15.md
- 06_bootstrap/S18.md
- 06_bootstrap/S02.md
- 06_bootstrap/S14.md
- 02_architecture/18_비용_장애_보안_운영.md
- 02_architecture/00_00_INDEX_02_architecture.md
- 02_architecture/10_API와_언어_간_계약.md
- 02_architecture/12_DB_제약_보존_마이그레이션.md
- 02_architecture/07_전체_프로그램_구성.md
- 02_architecture/09_Kotlin_백엔드_구조.md
- 02_architecture/17_CI_CD와_콘텐츠_배포.md
- 02_architecture/15_Rust_콘텐츠_도구와_Go_Ops.md
- 02_architecture/16_인프라와_컨테이너_배치.md
- 02_architecture/08_프론트엔드_구조.md
- 02_architecture/11_DB_논리_구조.md
- 00_guide/CONTRACTS.md
- 00_guide/BOOTSTRAP_EXAMPLES.md
- 00_guide/ZERO_START.md
- 00_guide/COMMANDS.md
- 00_guide/25_결정_기록과_공식_참고.md
- 00_guide/TASK_BOARD.md
- 00_guide/19_개발_로드맵.md
- 00_guide/DECISIONS.md
- 10_pc_profile/03_EXECUTION_PLAN.md
- 10_pc_profile/05_STORAGE_AND_MODEL_GATE.md
- 10_pc_profile/01_HARDWARE_BASELINE.md
- 10_pc_profile/02_RESOURCE_BUDGET.md

현재 추가 검토는 [[CHANGELOG_v5]]와 [[WF06_OPERATIONS]]를 참조합니다.
