//! 개념: schemaVersion 1 수정본 검사.
//! 실행: JSON 문자열을 읽어 오류를 모두 모은 뒤 반환한다. 첫 오류에서 멈추지 않는다.
//! 목적: 모르는 버전, 모르는 블록, 추가 필드를 저장본과 같은 코드로 거절한다.
//! 관련: `services/api/src/content.rs`의 `parse_save`, `validate_blocks`.

use serde_json::{Map, Value};

/// 개념: 호출자가 처리할 검사 오류.
/// 실행: path는 JSON 경로, code는 기계가 읽는 코드, message는 사람용 문장이다.
/// 목적: CLI가 이 값을 stderr JSON으로 바꾸고 0이 아닌 종료 코드로 나간다.
/// 관련: `exit_code`, `tools/content-cli`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentError {
    pub path: String,
    pub code: String,
    pub message: String,
    pub block_id: Option<String>,
}

/// 개념: 수정본 문자열 검사의 입구.
/// 실행: 파싱에 실패하면 invalid_json 하나만 돌려준다. 그 외에는 블록을 끝까지 본다.
/// 목적: 잘못된 입력이 조용히 통과하지 않게 한다.
/// 관련: `fixtures/contracts/m4-note.json`.
pub fn check_document(input: &str) -> Vec<ContentError> {
    let Ok(value) = serde_json::from_str::<Value>(input) else {
        return vec![error("", "invalid_json", "JSON으로 읽지 못했다", None)];
    };
    let Some(object) = value.as_object() else {
        return vec![error("", "invalid_json", "수정본은 객체여야 한다", None)];
    };
    let mut errors = Vec::new();
    reject_unknown(
        object,
        &["schemaVersion", "revisionId", "entryId", "createdAt", "title", "summary", "canonicalUrl", "blocks", "metadata"],
        "",
        None,
        &mut errors,
    );
    match object.get("schemaVersion").and_then(Value::as_i64) {
        Some(1) => {}
        _ => errors.push(error("schemaVersion", "unknown_version", "schemaVersion 1만 받는다", None)),
    }
    for key in ["revisionId", "entryId"] {
        match object.get(key).and_then(Value::as_str) {
            Some(value) if is_uuid(value) => {}
            _ => errors.push(error(key, "invalid", "UUID가 아니다", None)),
        }
    }
    bounded(object.get("title"), 1, 200, "title", None, &mut errors);
    if object.contains_key("summary") {
        bounded(object.get("summary"), 0, 500, "summary", None, &mut errors);
    }
    match object.get("blocks").and_then(Value::as_array) {
        Some(blocks) if blocks.len() <= 200 => validate_blocks(blocks, &mut errors),
        Some(_) => errors.push(error("blocks", "too_long", "블록은 200개를 넘기지 않는다", None)),
        None => errors.push(error("blocks", "required", "블록 배열이 없다", None)),
    }
    errors
}

fn validate_blocks(blocks: &[Value], errors: &mut Vec<ContentError>) {
    let mut seen_ids = Vec::new();
    let mut seen_anchors = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        let path = format!("blocks/{index}");
        let Some(object) = block.as_object() else {
            errors.push(error(&path, "invalid", "블록은 객체여야 한다", None));
            continue;
        };
        let block_id = object.get("id").and_then(Value::as_str).map(str::to_string);
        let kind = object.get("type").and_then(Value::as_str).unwrap_or("");
        match kind {
            "paragraph" => paragraph(object, &path, block_id.as_deref(), errors),
            "heading" => heading(object, &path, block_id.as_deref(), &mut seen_anchors, errors),
            "list" => list(object, &path, block_id.as_deref(), errors),
            "quote" => quote(object, &path, block_id.as_deref(), errors),
            "table" => table(object, &path, block_id.as_deref(), errors),
            "image" => image(object, &path, block_id.as_deref(), errors),
            "code" => code(object, &path, block_id.as_deref(), errors),
            "math" => math(object, &path, block_id.as_deref(), errors),
            "callout" => callout(object, &path, block_id.as_deref(), errors),
            "related" => related(object, &path, block_id.as_deref(), errors),
            _ => errors.push(error(&format!("{path}/type"), "unknown_type", "지원하지 않는 블록이다", block_id.as_deref())),
        }
        if let Some(id) = block_id.as_deref() {
            if seen_ids.iter().any(|seen: &String| seen == id) {
                errors.push(error(&format!("{path}/id"), "duplicate", "같은 블록 ID가 있다", Some(id)));
            }
            seen_ids.push(id.to_string());
        }
    }
}

fn paragraph(object: &Map<String, Value>, path: &str, block_id: Option<&str>, errors: &mut Vec<ContentError>) {
    reject_unknown(object, &["id", "type", "text"], path, block_id, errors);
    require_uuid(object, path, block_id, errors);
    bounded(object.get("text"), 1, 4000, &format!("{path}/text"), block_id, errors);
}

fn heading(object: &Map<String, Value>, path: &str, block_id: Option<&str>, seen_anchors: &mut Vec<String>, errors: &mut Vec<ContentError>) {
    reject_unknown(object, &["id", "type", "level", "text", "anchor"], path, block_id, errors);
    require_uuid(object, path, block_id, errors);
    match object.get("level").and_then(Value::as_i64) {
        Some(2) | Some(3) => {}
        _ => errors.push(error(&format!("{path}/level"), "invalid", "제목 단계는 2 또는 3이다", block_id)),
    }
    bounded(object.get("text"), 1, 200, &format!("{path}/text"), block_id, errors);
    let anchor = bounded(object.get("anchor"), 1, 80, &format!("{path}/anchor"), block_id, errors);
    if !anchor.is_empty() && !is_anchor(&anchor) {
        errors.push(error(&format!("{path}/anchor"), "invalid", "앵커는 소문자, 숫자, 하이픈만 쓴다", block_id));
    }
    if !anchor.is_empty() && seen_anchors.iter().any(|seen| seen == &anchor) {
        errors.push(error(&format!("{path}/anchor"), "duplicate_anchor", "같은 앵커가 이미 있다", block_id));
    }
    if !anchor.is_empty() {
        seen_anchors.push(anchor);
    }
}

fn list(object: &Map<String, Value>, path: &str, block_id: Option<&str>, errors: &mut Vec<ContentError>) {
    reject_unknown(object, &["id", "type", "ordered", "items"], path, block_id, errors);
    require_uuid(object, path, block_id, errors);
    if object.get("ordered").and_then(Value::as_bool).is_none() {
        errors.push(error(&format!("{path}/ordered"), "invalid", "목록의 ordered는 참 또는 거짓이다", block_id));
    }
    match object.get("items").and_then(Value::as_array) {
        Some(items) if (1..=50).contains(&items.len()) => {
            for (index, item) in items.iter().enumerate() {
                bounded(Some(item), 1, 500, &format!("{path}/items/{index}"), block_id, errors);
            }
        }
        _ => errors.push(error(&format!("{path}/items"), "invalid", "목록 항목은 1개에서 50개다", block_id)),
    }
}

fn quote(object: &Map<String, Value>, path: &str, block_id: Option<&str>, errors: &mut Vec<ContentError>) {
    reject_unknown(object, &["id", "type", "text", "citation"], path, block_id, errors);
    require_uuid(object, path, block_id, errors);
    bounded(object.get("text"), 1, 4000, &format!("{path}/text"), block_id, errors);
    if object.contains_key("citation") {
        bounded(object.get("citation"), 1, 300, &format!("{path}/citation"), block_id, errors);
    }
}

fn table(object: &Map<String, Value>, path: &str, block_id: Option<&str>, errors: &mut Vec<ContentError>) {
    reject_unknown(object, &["id", "type", "caption", "header", "rows"], path, block_id, errors);
    require_uuid(object, path, block_id, errors);
    if object.contains_key("caption") {
        bounded(object.get("caption"), 1, 200, &format!("{path}/caption"), block_id, errors);
    }
    match object.get("header").and_then(Value::as_array) {
        Some(header) if (1..=12).contains(&header.len()) => {
            for (index, cell) in header.iter().enumerate() {
                bounded(Some(cell), 1, 80, &format!("{path}/header/{index}"), block_id, errors);
            }
        }
        _ => errors.push(error(&format!("{path}/header"), "invalid", "표 머리는 1칸에서 12칸이다", block_id)),
    }
    match object.get("rows").and_then(Value::as_array) {
        Some(rows) if rows.len() <= 100 => {
            for (row_index, row) in rows.iter().enumerate() {
                match row.as_array() {
                    Some(cells) if (1..=12).contains(&cells.len()) => {
                        for (cell_index, cell) in cells.iter().enumerate() {
                            bounded(Some(cell), 0, 500, &format!("{path}/rows/{row_index}/{cell_index}"), block_id, errors);
                        }
                    }
                    _ => errors.push(error(&format!("{path}/rows/{row_index}"), "invalid", "표의 행이 배열이 아니다", block_id)),
                }
            }
        }
        _ => errors.push(error(&format!("{path}/rows"), "invalid", "표의 행이 없다", block_id)),
    }
}

fn image(object: &Map<String, Value>, path: &str, block_id: Option<&str>, errors: &mut Vec<ContentError>) {
    reject_unknown(object, &["id", "type", "alt", "url"], path, block_id, errors);
    require_uuid(object, path, block_id, errors);
    bounded(object.get("alt"), 1, 200, &format!("{path}/alt"), block_id, errors);
    let url = bounded(object.get("url"), 12, 500, &format!("{path}/url"), block_id, errors);
    if !url.is_empty() && !url.starts_with("https://") {
        errors.push(error(&format!("{path}/url"), "invalid", "이미지 주소는 https로 시작한다", block_id));
    }
}

fn code(object: &Map<String, Value>, path: &str, block_id: Option<&str>, errors: &mut Vec<ContentError>) {
    reject_unknown(object, &["id", "type", "language", "text"], path, block_id, errors);
    require_uuid(object, path, block_id, errors);
    bounded(object.get("language"), 1, 32, &format!("{path}/language"), block_id, errors);
    bounded(object.get("text"), 1, 20000, &format!("{path}/text"), block_id, errors);
}

fn math(object: &Map<String, Value>, path: &str, block_id: Option<&str>, errors: &mut Vec<ContentError>) {
    reject_unknown(object, &["id", "type", "tex"], path, block_id, errors);
    require_uuid(object, path, block_id, errors);
    bounded(object.get("tex"), 1, 4000, &format!("{path}/tex"), block_id, errors);
}

fn callout(object: &Map<String, Value>, path: &str, block_id: Option<&str>, errors: &mut Vec<ContentError>) {
    reject_unknown(object, &["id", "type", "tone", "text"], path, block_id, errors);
    require_uuid(object, path, block_id, errors);
    match object.get("tone").and_then(Value::as_str) {
        Some("note") | Some("warning") => {}
        _ => errors.push(error(&format!("{path}/tone"), "invalid", "콜아웃 톤은 note 또는 warning이다", block_id)),
    }
    bounded(object.get("text"), 1, 2000, &format!("{path}/text"), block_id, errors);
}

fn related(object: &Map<String, Value>, path: &str, block_id: Option<&str>, errors: &mut Vec<ContentError>) {
    reject_unknown(object, &["id", "type", "entryId"], path, block_id, errors);
    require_uuid(object, path, block_id, errors);
    match object.get("entryId").and_then(Value::as_str) {
        Some(value) if is_uuid(value) => {}
        _ => errors.push(error(&format!("{path}/entryId"), "invalid", "관련 글 ID가 UUID가 아니다", block_id)),
    }
}

fn reject_unknown(object: &Map<String, Value>, allowed: &[&str], path: &str, block_id: Option<&str>, errors: &mut Vec<ContentError>) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            let field_path = if path.is_empty() { key.clone() } else { format!("{path}/{key}") };
            errors.push(error(&field_path, "unknown_field", "계약에 없는 필드다", block_id));
        }
    }
}

fn require_uuid(object: &Map<String, Value>, path: &str, block_id: Option<&str>, errors: &mut Vec<ContentError>) {
    match object.get("id").and_then(Value::as_str) {
        Some(value) if is_uuid(value) => {}
        _ => errors.push(error(&format!("{path}/id"), "invalid", "블록 ID는 UUID다", block_id)),
    }
}

fn bounded(value: Option<&Value>, min: usize, max: usize, path: &str, block_id: Option<&str>, errors: &mut Vec<ContentError>) -> String {
    let Some(text) = value.and_then(Value::as_str) else {
        errors.push(error(path, "invalid", "문자열이 아니다", block_id));
        return String::new();
    };
    let length = text.chars().count();
    if length < min {
        errors.push(error(path, "too_short", "너무 짧다", block_id));
    } else if length > max {
        errors.push(error(path, "too_long", "너무 길다", block_id));
    }
    text.to_string()
}

fn error(path: &str, code: &str, message: &str, block_id: Option<&str>) -> ContentError {
    ContentError {
        path: path.to_string(),
        code: code.to_string(),
        message: message.to_string(),
        block_id: block_id.map(str::to_string),
    }
}

fn is_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    let groups = [8, 4, 4, 4, 12];
    let mut index = 0;
    for (group, length) in groups.iter().enumerate() {
        if group > 0 {
            if bytes.get(index) != Some(&b'-') {
                return false;
            }
            index += 1;
        }
        for _ in 0..*length {
            let Some(byte) = bytes.get(index) else {
                return false;
            };
            if !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase() {
                return false;
            }
            index += 1;
        }
    }
    index == bytes.len()
}

fn is_anchor(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > 80 || bytes[0] == b'-' || *bytes.last().unwrap_or(&b'-') == b'-' {
        return false;
    }
    let mut previous_dash = false;
    for byte in bytes {
        let allowed = byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-';
        if !allowed || (*byte == b'-' && previous_dash) {
            return false;
        }
        previous_dash = *byte == b'-';
    }
    true
}

#[cfg(test)]
mod tests {
    use super::check_document;

    /// 개념: 정상 수정본과 거절 케이스.
    /// 실행: 고정 JSON을 검사해 코드만 비교한다.
    /// 목적: 모르는 버전과 추가 필드, 모르는 블록이 통과하지 않는지 확인한다.
    /// 관련: `fixtures/contracts/m4-note.json`, `tools/content-core/check.mjs`.
    #[test]
    fn fixture_passes_and_unknown_values_are_rejected() {
        let fixture = include_str!("../../../fixtures/contracts/m4-note.json");
        assert!(check_document(fixture).is_empty());
        let unknown_version = fixture.replacen("\"schemaVersion\": 1", "\"schemaVersion\": 2", 1);
        assert!(check_document(&unknown_version).iter().any(|error| error.code == "unknown_version"));
        let extra = fixture.replacen("\"type\": \"paragraph\"", "\"type\": \"paragraph\", \"extra\": true", 1);
        assert!(check_document(&extra).iter().any(|error| error.code == "unknown_field"));
        let widget = fixture.replacen("\"type\": \"paragraph\"", "\"type\": \"widget\"", 1);
        assert!(check_document(&widget).iter().any(|error| error.code == "unknown_type"));
    }
}
