//! 개념: 블록을 빠뜨리지 않고 읽고 절 앵커를 뽑는다.
//! 실행: 검사에 통과한 문서만 순회한다. 이미지 alt는 본문 문자열과 다른 칸에 둔다.
//! 목적: 코드, 수식, 표, 목록이 조용히 사라지지 않게 한다.
//! 관련: `apps/web/src/site/blocks.tsx`의 heading id, `apps/admin/src/editor/Editor.tsx`의 `anchorFrom`.

use serde_json::Value;

/// 개념: 순회 결과의 한 블록.
/// 실행: body는 글 문장이고 alt는 이미지 대체 문장이다. 둘을 합치지 않는다.
/// 목적: 내보내기와 청크가 의미를 구분하게 한다.
/// 관련: `export_document`, `chunk_document`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkedBlock {
    pub index: usize,
    pub kind: String,
    pub block_id: String,
    pub body: String,
    pub alt: String,
}

/// 개념: 제목으로 나눈 절.
/// 실행: heading_path는 상위 제목부터 현재 제목까지다. anchor는 원문 앵커를 유지한다.
/// 목적: 같은 제목이 겹치면 뒤 절만 `-2`처럼 번호를 붙여 구분한다.
/// 관련: 공개 HTML의 `id` 속성은 이 함수가 다시 쓰지 않는다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub anchor: String,
    pub heading_path: Vec<String>,
    pub block_indexes: Vec<usize>,
}

/// 개념: 블록 배열을 순서대로 읽는다.
/// 실행: 배열이 아니면 빈 목록이다. 각 블록의 종류와 본문을 남긴다.
/// 목적: 깊은 목록의 항목 수도 본문에 남겨 누락을 드러낸다.
/// 관련: `validate::check_document`.
pub fn walk_blocks(input: &str) -> Vec<WalkedBlock> {
    let Ok(value) = serde_json::from_str::<Value>(input) else {
        return Vec::new();
    };
    let Some(blocks) = value.get("blocks").and_then(Value::as_array) else {
        return Vec::new();
    };
    blocks.iter().enumerate().map(|(index, block)| {
        let kind = block.get("type").and_then(Value::as_str).unwrap_or("").to_string();
        let block_id = block.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        let body = match kind.as_str() {
            "list" => block.get("items").and_then(Value::as_array).map(|items| items.len().to_string()).unwrap_or_default(),
            "table" => block.get("caption").and_then(Value::as_str).unwrap_or("").to_string(),
            "code" | "math" | "paragraph" | "quote" | "callout" | "heading" => text_of(block),
            _ => String::new(),
        };
        let alt = if kind == "image" { block.get("alt").and_then(Value::as_str).unwrap_or("").to_string() } else { String::new() };
        WalkedBlock { index, kind, block_id, body, alt }
    }).collect()
}

/// 개념: 제목 경로와 앵커를 뽑는다.
/// 실행: 제목이 없으면 anchor `section` 인 빈 절 하나를 만든다. 같은 앵커는 두 번째부터 `-2`를 붙인다.
/// 목적: 원문에 있는 유일한 앵커는 그대로 두고, 빈 절도 결과에서 지우지 않는다.
/// 관련: 편집기 `anchorFrom`은 한글 제목을 저장 시 `section`으로 바꾸지만, 저장된 anchor 필드가 있으면 그 값을 쓴다.
pub fn extract_sections(input: &str) -> Vec<Section> {
    let Ok(value) = serde_json::from_str::<Value>(input) else {
        return Vec::new();
    };
    let Some(blocks) = value.get("blocks").and_then(Value::as_array) else {
        return vec![empty_section()];
    };
    let mut sections = Vec::new();
    let mut path: Vec<String> = Vec::new();
    let mut current: Option<Section> = None;
    let mut used = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        if block.get("type").and_then(Value::as_str) == Some("heading") {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            let level = block.get("level").and_then(Value::as_i64).unwrap_or(2);
            let text = block.get("text").and_then(Value::as_str).unwrap_or("").to_string();
            if level <= 2 {
                path.clear();
            } else if path.len() > 1 {
                path.truncate(1);
            }
            path.push(text);
            let source = block.get("anchor").and_then(Value::as_str).unwrap_or("section");
            let anchor = unique_anchor(source, &mut used);
            current = Some(Section { anchor, heading_path: path.clone(), block_indexes: vec![index] });
        } else if let Some(section) = current.as_mut() {
            section.block_indexes.push(index);
        } else {
            current = Some(Section { anchor: unique_anchor("section", &mut used), heading_path: Vec::new(), block_indexes: vec![index] });
        }
    }
    if let Some(section) = current {
        sections.push(section);
    }
    if sections.is_empty() {
        sections.push(empty_section());
    }
    sections
}

fn empty_section() -> Section {
    Section { anchor: "section".to_string(), heading_path: Vec::new(), block_indexes: Vec::new() }
}

fn unique_anchor(source: &str, used: &mut Vec<String>) -> String {
    if !used.iter().any(|anchor| anchor == source) {
        used.push(source.to_string());
        return source.to_string();
    }
    let mut number = 2;
    loop {
        let candidate = format!("{source}-{number}");
        if !used.iter().any(|anchor| anchor == &candidate) {
            used.push(candidate.clone());
            return candidate;
        }
        number += 1;
    }
}

fn text_of(block: &Value) -> String {
    for key in ["text", "tex"] {
        if let Some(text) = block.get(key).and_then(Value::as_str) {
            return text.to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::{extract_sections, walk_blocks};

    /// 개념: 앵커와 누락 방지.
    /// 실행: 고정 문서의 앵커가 원문과 같은지, 빈 문서와 중복 제목과 깊은 목록을 확인한다.
    /// 목적: 공개 페이지의 id와 추출 앵커가 같은 문자열을 쓰는지 고정한다.
    /// 관련: `apps/web/src/site/blocks.tsx`.
    #[test]
    fn anchors_match_source_and_blocks_stay_present() {
        let fixture = include_str!("../../../fixtures/contracts/m4-note.json");
        let walked = walk_blocks(fixture);
        assert_eq!(walked.len(), 6);
        assert_eq!(walked[4].alt, "표지");
        assert_eq!(walked[1].body, "저장한 문장");
        assert!(walked[1].alt.is_empty());
        let sections = extract_sections(fixture);
        assert_eq!(sections[0].anchor, "field-note");
        assert_eq!(sections[0].heading_path, vec!["현장 메모".to_string()]);

        let empty = r#"{"schemaVersion":1,"blocks":[]}"#;
        assert_eq!(extract_sections(empty)[0].anchor, "section");
        assert!(extract_sections(empty)[0].block_indexes.is_empty());

        let duplicate = r#"{"blocks":[
            {"type":"heading","level":2,"text":"가","anchor":"same"},
            {"type":"heading","level":2,"text":"나","anchor":"same"}
        ]}"#;
        let found = extract_sections(duplicate);
        assert_eq!(found[0].anchor, "same");
        assert_eq!(found[1].anchor, "same-2");

        let deep = r#"{"blocks":[{"type":"list","items":["가","나","다"]}]}"#;
        assert_eq!(walk_blocks(deep)[0].body, "3");
    }
}
