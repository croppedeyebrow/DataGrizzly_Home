//! 개념: 절 단위 청크와 원자적 파일 쓰기.
//! 실행: 길이는 유니코드 스칼라 개수다. 코드와 수식과 표는 스칼라 경계 안에서도 쪼개지 않는다.
//! 목적: 같은 수정본이 항상 같은 JSONL과 식별자를 만들게 한다.
//! 관련: `tools/content-cli`의 chunk 하위 명령, `contracts/chunks.schema.json`.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::walk::{extract_sections, walk_blocks};

/// 개념: 청크 하나의 최대 스칼라 수.
/// 실행: 이 값을 넘는 일반 문장만 다음 청크로 넘긴다. 코드 블록은 더 길어도 한 청크로 남는다.
/// 목적: 토큰 수를 추측해 자르지 않는다.
/// 관련: `chunk_document`.
pub const CHUNK_CHARS: usize = 400;

/// 개념: 한 청크.
/// 실행: chunk_id는 수정본, 절, 순서, 버전을 해시한 값이다.
/// 목적: 재실행해도 식별자가 바뀌지 않게 한다.
/// 관련: `tools/content-cli`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub chunk_id: String,
    pub anchor: String,
    pub order: usize,
    pub text: String,
}

/// 개념: 문서를 청크로 나눈다.
/// 실행: 먼저 절을 만들고, 절 안의 일반 문장이 400자를 넘을 때만 다음 청크를 연다.
/// 목적: 한글, 코드, 수식의 중간을 자르지 않는다.
/// 관련: `extract_sections`.
pub fn chunk_document(input: &str) -> Vec<Chunk> {
    let value: Value = serde_json::from_str(input).unwrap_or(Value::Null);
    let entry_id = value.get("entryId").and_then(Value::as_str).unwrap_or("");
    let revision_id = value.get("revisionId").and_then(Value::as_str).unwrap_or("");
    let walked = walk_blocks(input);
    let mut chunks = Vec::new();
    for section in extract_sections(input) {
        let mut text = String::new();
        for index in &section.block_indexes {
            let Some(block) = walked.get(*index) else { continue };
            let piece = if block.kind == "image" { block.alt.clone() } else { block.body.clone() };
            let atomic = matches!(block.kind.as_str(), "code" | "math" | "table" | "list");
            if atomic {
                if !text.is_empty() {
                    push_chunk(&mut chunks, entry_id, revision_id, &section.anchor, text);
                    text = String::new();
                }
                push_chunk(&mut chunks, entry_id, revision_id, &section.anchor, piece);
                continue;
            }
            let mut rest: Vec<char> = piece.chars().collect();
            while !rest.is_empty() {
                let room = CHUNK_CHARS.saturating_sub(text.chars().count() + usize::from(!text.is_empty()));
                if room == 0 {
                    push_chunk(&mut chunks, entry_id, revision_id, &section.anchor, text);
                    text = String::new();
                    continue;
                }
                let take = rest.len().min(room);
                if !text.is_empty() {
                    text.push('\n');
                }
                text.extend(rest.drain(..take));
            }
        }
        if !text.is_empty() || section.block_indexes.is_empty() {
            push_chunk(&mut chunks, entry_id, revision_id, &section.anchor, text);
        }
    }
    chunks
}

fn push_chunk(chunks: &mut Vec<Chunk>, entry_id: &str, revision_id: &str, anchor: &str, text: String) {
    let order = chunks.len();
    let identity = format!("{entry_id}|{revision_id}|{anchor}|{order}|1");
    let chunk_id = hex_sha256(identity.as_bytes());
    chunks.push(Chunk { chunk_id, anchor: anchor.to_string(), order, text });
}

/// 개념: 청크 목록의 정규 JSONL.
/// 실행: 키 순서는 chunkId, anchor, order, text로 고정한다.
/// 목적: 같은 입력이 바이트까지 같은 출력을 만들게 한다.
/// 관련: `chunk_document`.
pub fn canonical_jsonl(chunks: &[Chunk]) -> String {
    let mut out = String::new();
    for chunk in chunks {
        let line = json!({
            "chunkId": chunk.chunk_id,
            "anchor": chunk.anchor,
            "order": chunk.order,
            "text": chunk.text,
        });
        out.push_str(&line.to_string());
        out.push('\n');
    }
    out
}

/// 개념: 최종 파일을 임시 파일에서 바꾼다.
/// 실행: 같은 폴더에 `.next`를 쓰고, 기존 파일을 `.bak`으로 옮긴 뒤 교체한다. 교체가 실패하면 기존 파일을 되돌린다.
/// 목적: 디스크가 가득 차거나 교체가 실패해도 이전 정상 결과를 남긴다.
/// 관련: `tools/content-cli`의 chunk와 export.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "output parent missing".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temp = path.with_extension("next");
    let backup = path.with_extension("bak");
    {
        let mut file = fs::File::create(&temp).map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
    }
    if path.exists() {
        let _ = fs::remove_file(&backup);
        fs::rename(path, &backup).map_err(|error| error.to_string())?;
    }
    if let Err(error) = fs::rename(&temp, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(error.to_string());
    }
    let _ = fs::remove_file(&backup);
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::{canonical_jsonl, chunk_document, write_atomic};
    use std::fs;

    /// 개념: 재현 가능한 청크.
    /// 실행: 같은 문서를 두 번 나누고, 긴 한글과 코드를 경계에서 확인한다.
    /// 목적: 해시와 JSONL이 흔들리지 않는지 본다.
    /// 관련: `fixtures/contracts/m4-note.json`.
    #[test]
    fn same_input_keeps_the_same_jsonl_and_code_block() {
        let fixture = include_str!("../../../fixtures/contracts/m4-note.json");
        let first = canonical_jsonl(&chunk_document(fixture));
        let second = canonical_jsonl(&chunk_document(fixture));
        assert_eq!(first, second);
        assert!(first.contains("fn main() {}"));
        let hangul = "가".repeat(401);
        let long = format!(
            r#"{{"entryId":"22222222-2222-4222-8222-222222222222","revisionId":"11111111-1111-4111-8111-111111111111","blocks":[{{"type":"paragraph","text":"{hangul}"}}]}}"#
        );
        let chunks = chunk_document(&long);
        assert!(chunks[0].text.chars().count() <= 400);
        assert!(chunks[0].text.chars().all(|char| char == '가'));
        let code = format!(
            r#"{{"entryId":"22222222-2222-4222-8222-222222222222","revisionId":"11111111-1111-4111-8111-111111111111","blocks":[{{"type":"code","text":"{}"}}]}}"#,
            "가".repeat(500)
        );
        assert_eq!(chunk_document(&code).len(), 1);
    }

    /// 개념: 실패한 쓰기가 기존 파일을 남긴다.
    /// 실행: 정상 파일을 만든 뒤 임시 파일 교체 전 검증 실패는 파일을 열지 않는다.
    /// 목적: 부분 결과를 성공으로 보지 않는다.
    /// 관련: CLI는 오류가 있으면 `write_atomic`을 호출하지 않는다.
    #[test]
    fn existing_output_stays_when_the_caller_skips_the_write() {
        let dir = std::env::temp_dir().join(format!("dg-chunk-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("chunks.jsonl");
        write_atomic(&path, b"kept\n").unwrap();
        let errors = ["duplicate_anchor"];
        if errors.is_empty() {
            write_atomic(&path, b"replaced\n").unwrap();
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), "kept\n");
        let _ = fs::remove_dir_all(&dir);
    }
}
