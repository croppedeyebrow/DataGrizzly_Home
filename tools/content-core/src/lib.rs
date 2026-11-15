//! 개념: 수정본 JSON을 검사하고 절과 청크로 나눈다.
//! 실행: `check_document`가 오류 목록을 돌려준다. 비어 있으면 통과다. 이 함수는 프로세스를 끝내거나 파일을 쓰지 않는다.
//! 목적: API 저장 검사와 같은 거절 규칙을 CLI와 내보내기가 다시 쓰게 한다.
//! 관련: `services/api/src/content.rs`, `apps/admin/src/editor/Editor.tsx`, `tools/content-cli`.

mod chunk;
mod export;
mod normalize;
mod validate;
mod walk;

pub use chunk::{canonical_jsonl, chunk_document, write_atomic, Chunk, CHUNK_CHARS};
pub use export::{export_document, safe_relative, ExportFile};
pub use normalize::normalize_document;
pub use validate::{check_document, ContentError};
pub use walk::{extract_sections, walk_blocks, Section, WalkedBlock};

/// 개념: 라이브러리 오류를 CLI 종료 코드로 바꾼다.
/// 실행: 오류가 하나라도 있으면 1, 없으면 0이다. 호출한 쪽이 `std::process::exit`를 결정한다.
/// 목적: 검사 라이브러리가 프로그램을 죽이지 않게 한다.
/// 관련: `tools/content-cli/src/main.rs`.
pub fn exit_code(errors: &[ContentError]) -> i32 {
    if errors.is_empty() { 0 } else { 1 }
}
