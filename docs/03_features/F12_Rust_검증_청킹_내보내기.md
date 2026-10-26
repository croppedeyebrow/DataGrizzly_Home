# F12 Rust 검증 청킹 내보내기

> v5.0 기준: [[SCOPE_v5]] 적용. 주력 Kotlin·Rust·TypeScript·PostgreSQL. LLM·C++ 제외, Go는 국소 배포 도구로 필요할 때만 사용합니다. 로컬 자원은 [[02_RESOURCE_BUDGET]] 참조.
상태: 설계 / 구현 전. 기준: DataGrizzly v2.0. 세부 보완은 본 문서 세트 v2.1에서 제안합니다.

## 사용자와 범위

관리자·운영자가 사용하는 기능입니다. 아래 수용 조건을 구현 단위로 사용합니다.

## 화면·접점

`CLI dg-content / optional WASM`

## 데이터·계약

Document JSON → chunk JSONL + manifest; Markdown + block JSON + asset map

## 수용 조건

- 동일 입력 재현 가능
- 한글·코드·수식 경계 보존
- WASM 실패 시 서버 경로 유지

## 실패·보안·운영 조건

지원하지 않는 schemaVersion 거부; Markdown 완벽한 왕복 변환을 약속하지 않음

## 연결 개발 태스크

- [M4-01](../04_tasks/M4-01.md)
- [M4-02](../04_tasks/M4-02.md)
- [M4-03](../04_tasks/M4-03.md)
- [M4-04](../04_tasks/M4-04.md)

## 구현 시 검증

정상 입력, 비어 있는 데이터, 실패 상태를 각각 확인합니다. 인증·발행·파일·검색 경계를 변경하면 비공개 데이터의 익명 접근 및 재노출을 함께 검증합니다. 화면 캡처만으로 서버 권한과 데이터 보존 검증을 대체하지 않습니다.
