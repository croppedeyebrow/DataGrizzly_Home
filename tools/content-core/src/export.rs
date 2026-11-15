//! 개념: 같은 수정본의 Markdown, 원본 블록, 메타데이터, 첨부 대응.
//! 실행: Markdown은 읽기용이고 원본 JSON은 그대로 둔다. 경로가 폴더 밖을 가리키면 오류다.
//! 목적: 코드와 수식과 표가 Markdown에서 빠져도 원본 JSON에 남게 한다.
//! 관련: `infra/publish/build-candidate.mjs`는 검사 실패 뒤 이 결과를 성공으로 쓰지 않는다.

use serde_json::{json, Value};

use crate::validate::{check_document, ContentError};
use crate::walk::walk_blocks;

/// 개념: 내보내기 파일 하나.
/// 실행: name은 출력 폴더 안의 상대 이름이다.
/// 목적: CLI가 이 목록을 원자적으로 쓰게 한다.
/// 관련: `tools/content-cli`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// 개념: 상대 경로가 출력 폴더 안에 있는지 본다.
/// 실행: 빈 값, 절대 경로, `..`, 역슬래시는 거절한다.
/// 목적: 첨부 대응이 다른 폴더의 파일을 가리키지 않게 한다.
/// 관련: `export_document`.
pub fn safe_relative(path: &str) -> Result<String, &'static str> {
    if path.is_empty() || path.starts_with('/') || path.starts_with('\\') || path.contains(':') || path.contains('\\') || path.split(['/', '\\']).any(|part| part == "..") {
        return Err("path_escape");
    }
    Ok(path.to_string())
}

/// 개념: 수정본을 네 파일로 나눈다.
/// 실행: 검사가 실패하면 파일을 만들지 않고 오류만 돌려준다. 없는 로컬 파일도 오류다.
/// 목적: 부분 내보내기를 성공으로 기록하지 않는다.
/// 관련: `check_document`.
pub fn export_document(input: &str, files_present: &[&str]) -> Result<Vec<ExportFile>, Vec<ContentError>> {
    let errors = check_document(input);
    if !errors.is_empty() {
        return Err(errors);
    }
    let value: Value = serde_json::from_str(input).unwrap_or(Value::Null);
    let mut asset_errors = Vec::new();
    let mut assets = Vec::new();
    if let Some(blocks) = value.get("blocks").and_then(Value::as_array) {
        for block in blocks {
            if block.get("type").and_then(Value::as_str) != Some("image") {
                continue;
            }
            let source = block.get("url").and_then(Value::as_str).unwrap_or("");
            let block_id = block.get("id").and_then(Value::as_str).unwrap_or("");
            if source.starts_with("https://") {
                assets.push(json!({"blockId": block_id, "source": source, "target": "", "status": "remote"}));
                continue;
            }
            match safe_relative(source) {
                Ok(target) if files_present.contains(&target.as_str()) => {
                    assets.push(json!({"blockId": block_id, "source": source, "target": target, "status": "mapped"}));
                }
                Ok(target) => {
                    asset_errors.push(ContentError { path: "blocks/url".to_string(), code: "missing_asset".to_string(), message: "첨부 파일이 없다".to_string(), block_id: Some(block_id.to_string()) });
                    assets.push(json!({"blockId": block_id, "source": source, "target": target, "status": "missing"}));
                }
                Err(code) => {
                    asset_errors.push(ContentError { path: "blocks/url".to_string(), code: code.to_string(), message: "첨부 경로가 폴더 밖을 가리킨다".to_string(), block_id: Some(block_id.to_string()) });
                }
            }
        }
    }
    if !asset_errors.is_empty() {
        return Err(asset_errors);
    }
    let markdown = markdown_of(input);
    let metadata = value.get("metadata").cloned().unwrap_or(Value::Null).to_string();
    let blocks = value.get("blocks").cloned().unwrap_or(Value::Null).to_string();
    Ok(vec![
        ExportFile { name: "page.md".to_string(), bytes: markdown.into_bytes() },
        ExportFile { name: "blocks.json".to_string(), bytes: blocks.into_bytes() },
        ExportFile { name: "metadata.json".to_string(), bytes: metadata.into_bytes() },
        ExportFile { name: "assets.json".to_string(), bytes: Value::Array(assets).to_string().into_bytes() },
    ])
}

fn markdown_of(input: &str) -> String {
    let mut lines = Vec::new();
    for block in walk_blocks(input) {
        let line = match block.kind.as_str() {
            "heading" => format!("## {}", block.body),
            "paragraph" | "quote" | "callout" => block.body,
            "code" => format!("```\n{}\n```", block.body),
            "math" => format!("$$\n{}\n$$", block.body),
            "image" => format!("![{}](remote)", block.alt),
            "list" => format!("- items: {}", block.body),
            "table" => format!("table: {}", block.body),
            _ => String::new(),
        };
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::{export_document, safe_relative};

    /// 개념: 경로와 원본 보존.
    /// 실행: 상위 경로를 거절하고, 정상 문서의 Markdown과 원본 JSON을 확인한다.
    /// 목적: 수식이 Markdown에만 있고 원본에서 사라지지 않는지 본다.
    /// 관련: `fixtures/contracts/m4-note.json`.
    #[test]
    fn export_keeps_original_json_and_rejects_escape() {
        assert!(safe_relative("../secret").is_err());
        assert!(safe_relative("images/a.png").is_ok());
        let fixture = include_str!("../../../fixtures/contracts/m4-note.json");
        let files = export_document(fixture, &[]).expect("export");
        let markdown = String::from_utf8(files[0].bytes.clone()).unwrap();
        let blocks = String::from_utf8(files[1].bytes.clone()).unwrap();
        assert!(markdown.contains("E=mc^2"));
        assert!(blocks.contains("\"tex\""));
        assert!(markdown.contains("표지"));
    }
}
