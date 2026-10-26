# 스키마 버전과 소비자

- schemaVersion: **1**
- 기계 계약: `contracts/content.schema.json`, `contracts/openapi.yaml`
- 예시: `fixtures/contracts/`
- 사람용 경로 계약: `docs/contracts/routes.md`, `docs/contracts/scope.md`

## 지원 버전

schemaVersion 1만 받는다. 없거나 1이 아니면 거절하고 자동으로 고치지 않는다. 알 수 없는 필드와 블록 type도 거절한다. 필드를 추가하거나 블록을 바꾸면 schemaVersion을 올리고, 생산자가 새 버전을 보내기 전에 이전 기록을 남긴다. 1은 첫 버전이므로 이전 버전에서 넘어오는 migration 파일은 없다.

길이 상한은 문자 수다. 토큰 수가 아니다. 운영에서 바꾼 값은 스키마와 테스트를 함께 바꾼다.

## 생산자와 소비자

Kotlin CMS, TypeScript 관리자·공개 웹, Rust 콘텐츠 도구는 이 저장소의 같은 파일 경로를 읽는다. `packages/content-schema` 래퍼는 아직 만들지 않았다. 세 언어의 구현 코드도 아직 없으므로, 지금은 경로만 고정하고 언어별 실행은 하지 않는다.

| 소비자 | 읽을 경로 | 구현 |
|---|---|---|
| Kotlin CMS | `contracts/content.schema.json`, `contracts/openapi.yaml` | 없음. M2 |
| TypeScript | 같은 스키마와 `fixtures/contracts/` | 없음. 관리자는 M2, 공개 페이지는 M1 |
| Rust CLI | `contracts/content.schema.json`, `fixtures/contracts/` | 없음. M4 |

## 수정본과 공개 포인터

Revision 본문은 저장 후 바꾸지 않는다. 다시 저장하면 새 revisionId를 만든다. Entry.version은 낙관적 잠금 값이다. draftRevisionId와 publishedRevisionId는 다른 필드다. 초안 저장 응답의 version은 공개 releaseId가 아니다.

## 검증을 섞지 않는 기준

JSON Schema와 앵커 중복·표 열 수 검사는 문서 구조만 본다. projectId가 형식은 맞지만 알려진 목록에 없는 경우는 참조 검사에서만 실패한다. 그 목록은 fixture이고 데이터베이스 조회가 아니다.

블록 편집기 제품은 아직 정하지 않았다. 선택한 편집기는 이 스키마의 블록만 내보내야 한다. 수식은 TeX 문자열, 코드는 실행하지 않는 텍스트다. MDX와 script 블록은 없다.

## 아직 아닌 것

API 서버, 인증 제공자, DB 제약, Rust 검증 바이너리는 이 계약의 구현이 아니다.
