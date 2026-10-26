# 와이어프레임과 UI 이미지 모음

- wireframes/: 공개15개+관리자12개 =27개 PNG, 1440×1040.
- ui/: 같은27개 화면의 색상 적용 UI 레이아웃 PNG.
- mobile/: 핵심 모바일6개 PNG, 430×1000.
- concepts/brand_concept.png: 이미지 생성 도구로 제작한 브랜드 컨셉1개.
- *_overview.png: 각 폴더를 한눈에 보는 모음 이미지3개.

총64개 PNG입니다. 컨셉 이미지는 실제 화면 구현이 아닙니다. 원본 자료에서 가져온 확정 로고나 기존 SNS 디자인 복제본도 아닙니다. 와이어프레임/레이아웃은 정확한 라벨 배치를 위해 코드로 제작했고, 생성형 컨셉은 분위기를 제안합니다.

## 화면별 보기

- [P01 홈](../09_screen_specs/P01.md)
- [P02 소개](../09_screen_specs/P02.md)
- [P03 프로젝트 목록](../09_screen_specs/P03.md)
- [P04 프로젝트 설명](../09_screen_specs/P04.md)
- [P05 프로젝트 과정 목록](../09_screen_specs/P05.md)
- [P06 과정 기록 상세](../09_screen_specs/P06.md)
- [P07 노트 목록](../09_screen_specs/P07.md)
- [P08 노트 상세](../09_screen_specs/P08.md)
- [P09 리서치 목록](../09_screen_specs/P09.md)
- [P10 논문 리뷰 상세](../09_screen_specs/P10.md)
- [P11 뉴스 정리 상세](../09_screen_specs/P11.md)
- [P12 검색과 선택적 질문](../09_screen_specs/P12.md)
- [P13 주제별 목록](../09_screen_specs/P13.md)
- [P14 시리즈 상세](../09_screen_specs/P14.md)
- [P15 빈 결과 · 오류 · 404](../09_screen_specs/P15.md)
- [A01 관리자 로그인](../09_screen_specs/A01.md)
- [A02 관리 홈](../09_screen_specs/A02.md)
- [A03 게시물 관리](../09_screen_specs/A03.md)
- [A04 블록 편집기](../09_screen_specs/A04.md)
- [A05 미리보기](../09_screen_specs/A05.md)
- [A06 수정 이력과 복원](../09_screen_specs/A06.md)
- [A07 프로젝트 관리](../09_screen_specs/A07.md)
- [A08 분류와 시리즈 관리](../09_screen_specs/A08.md)
- [A09 미디어 관리](../09_screen_specs/A09.md)
- [A10 발행 작업](../09_screen_specs/A10.md)
- [A11 검색·모델 상태](../09_screen_specs/A11.md)
- [A12 사이트 설정](../09_screen_specs/A12.md)

## 모바일 패턴

MB01 홈, MB02 프로젝트, MB03 글 상세, MB04 탐색, MB05 편집기, MB06 발행 상태입니다. 데스크톱의 모든 화면을 별도 모바일 이미지로 복제한 것은 아니며 나머지는 UI/UX 반응형 규칙을 적용합니다.

## 제작 프롬프트 요약

내장 이미지 생성 도구 사용. DataGrizzly의 아이보리·숲녹색 편집형 UI, 홈/프로젝트 설명·과정 기록/CMS 편집기3개 패널, 브랜드 문구와 예시 표기. 최종 수정에서 프로젝트를 DataGrizzly Web으로 통일하고 Astro/React/Kotlin/PostgreSQL/Rust/선택 Go만 표기, 항공기 사진을 문서·시스템 일러스트로 교체했습니다. 생성형 작은 글자·도식은 계약 기준으로 쓰지 않습니다.


## v5.0 사용 범위

PNG64개는기존디자인참고로보존합니다. P12질문패널·A11모델상태·브랜드컨셉의C++배지는폐기된기능표현입니다. 새구현은TypeScript/Kotlin/Rust/PostgreSQL과검색전용UI를사용합니다. 이미지파일은실행아키텍처의근거가아닙니다.

## v5.0 자료 워크플로 반영

기존 PNG 64개는 보존합니다. v5 신규 관리자 화면 [[A13]]~[[A15]]는 명세만 추가했으며 새 이미지 파일은 아직 없습니다.
