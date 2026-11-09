//! 개념: 글 수정본의 본문 검사.
//! 목적: 편집기 상태를 계약된 schemaVersion 1 블록으로만 저장한다.
//! 사용처: 글 생성과 `PUT /api/admin/entries/{id}`.

use serde_json::{json, Map, Value};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use uuid::Uuid;

const STAMP: &[FormatItem<'_>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");

/// 개념: 저장 요청에서 거절한 필드.
/// 목적: 화면이 어느 블록이 틀렸는지 알게 한다.
/// 사용처: 422 응답의 fieldErrors.
#[derive(Debug, PartialEq, Eq)]
pub struct FieldError {
    pub path: String,
    pub code: String,
}

/// 개념: 한 번의 초안 저장 입력.
/// 목적: 제목과 블록을 버전 검사 전에 확정한다.
/// 사용처: `parse_save`.
#[derive(Debug)]
pub struct SaveInput {
    pub expected_version: i32,
    pub title: String,
    pub summary: String,
    pub blocks: Vec<Value>,
    pub metadata: Value,
}

/// 개념: 지금 시각을 계약의 UTC 문자열로 만든다.
/// 목적: 수정본 createdAt에 소수 초나 오프셋을 넣지 않는다.
/// 사용처: `revision_document`.
pub fn utc_stamp() -> String {
    OffsetDateTime::now_utc()
        .format(STAMP)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

/// 개념: 초안 문서에 넣을 https 주소.
/// 목적: 도메인이 비어 있는 동안에도 수정본 계약의 canonicalUrl을 채운다.
/// 사용처: 초안 생성과 저장. 공개 페이지의 기준 주소는 바꾸지 않는다.
pub fn canonical_url(kind: &str, slug: &str) -> String {
    let section = match kind {
        "log" => "logs",
        "paper" | "news" => "research",
        _ => "notes",
    };
    format!("https://datagrizzly.local/{section}/{slug}")
}

/// 개념: 새 글의 메타데이터.
/// 목적: 종류와 맞지 않는 메타데이터를 첫 수정본에 넣지 않는다.
/// 사용처: 글 생성. 로그의 빈 프로젝트는 0 UUID로 두고 화면에서 비어 있음을 보여 준다.
pub fn initial_metadata(kind: &str) -> Value {
    match kind {
        "paper" => json!({ "type": "paper" }),
        "news" => json!({ "type": "news" }),
        "log" => json!({
            "type": "log",
            "projectId": "00000000-0000-0000-0000-000000000000",
            "stage": "planning",
            "displayOrder": 0
        }),
        _ => json!({ "type": "note" }),
    }
}

/// 개념: 저장할 수정본 객체.
/// 목적: 서버가 revisionId, entryId, createdAt, schemaVersion을 채운다.
/// 사용처: revision.body.
pub fn revision_document(
    revision_id: Uuid,
    entry_id: Uuid,
    title: &str,
    canonical: &str,
    blocks: &[Value],
    metadata: &Value,
) -> Value {
    json!({
        "schemaVersion": 1,
        "revisionId": revision_id.to_string(),
        "entryId": entry_id.to_string(),
        "createdAt": utc_stamp(),
        "title": title,
        "canonicalUrl": canonical,
        "blocks": blocks,
        "metadata": metadata,
    })
}

/// 개념: 저장 JSON을 읽는다.
/// 목적: 빠진 필드와 블록 형식 오류를 저장 전에 모은다.
/// 사용처: `save_entry`.
pub fn parse_save(body: &str, kind: &str) -> Result<SaveInput, Vec<FieldError>> {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return Err(vec![field("", "invalid_json")]);
    };
    let Some(object) = value.as_object() else {
        return Err(vec![field("", "invalid_json")]);
    };
    let mut errors = Vec::new();
    reject_unknown(
        object,
        &["expectedVersion", "title", "summary", "blocks", "metadata"],
        "",
        &mut errors,
    );
    let expected_version = match object.get("expectedVersion").and_then(Value::as_i64) {
        Some(version) if (1..=1_000_000).contains(&version) => version as i32,
        _ => {
            errors.push(field("expectedVersion", "invalid"));
            0
        }
    };
    let title = bounded_string(object.get("title"), 1, 200, "title", &mut errors);
    let summary = bounded_string(object.get("summary"), 0, 500, "summary", &mut errors);
    let blocks = match object.get("blocks").and_then(Value::as_array) {
        Some(blocks) if blocks.len() <= 200 => blocks.clone(),
        Some(_) => {
            errors.push(field("blocks", "too_long"));
            Vec::new()
        }
        None => {
            errors.push(field("blocks", "required"));
            Vec::new()
        }
    };
    let metadata = object
        .get("metadata")
        .cloned()
        .unwrap_or_else(|| Value::Null);
    validate_blocks(&blocks, &mut errors);
    validate_metadata(kind, &metadata, &mut errors);
    if errors.is_empty() {
        Ok(SaveInput {
            expected_version,
            title,
            summary,
            blocks,
            metadata,
        })
    } else {
        Err(errors)
    }
}

/// 개념: 블록 배열을 계약 타입별로 검사한다.
/// 목적: 모르는 종류와 추가 필드를 원본으로 저장하지 않는다.
/// 사용처: `parse_save`.
fn validate_blocks(blocks: &[Value], errors: &mut Vec<FieldError>) {
    let mut seen = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        let path = format!("blocks/{index}");
        let Some(object) = block.as_object() else {
            errors.push(field(&path, "invalid"));
            continue;
        };
        let kind = object.get("type").and_then(Value::as_str).unwrap_or("");
        match kind {
            "paragraph" => validate_paragraph(object, &path, errors),
            "heading" => validate_heading(object, &path, errors),
            "list" => validate_list(object, &path, errors),
            "quote" => validate_quote(object, &path, errors),
            "table" => validate_table(object, &path, errors),
            "image" => validate_image(object, &path, errors),
            "code" => validate_code(object, &path, errors),
            "math" => validate_math(object, &path, errors),
            "callout" => validate_callout(object, &path, errors),
            "related" => validate_related(object, &path, errors),
            _ => errors.push(field(&format!("{path}/type"), "unknown_type")),
        }
        if let Some(id) = object.get("id").and_then(Value::as_str) {
            if seen.contains(&id.to_string()) {
                errors.push(field(&format!("{path}/id"), "duplicate"));
            }
            seen.push(id.to_string());
        }
    }
}

fn validate_paragraph(object: &Map<String, Value>, path: &str, errors: &mut Vec<FieldError>) {
    reject_unknown(object, &["id", "type", "text"], path, errors);
    require_uuid(object, path, errors);
    bounded_string(object.get("text"), 1, 4000, &format!("{path}/text"), errors);
}

fn validate_heading(object: &Map<String, Value>, path: &str, errors: &mut Vec<FieldError>) {
    reject_unknown(object, &["id", "type", "level", "text", "anchor"], path, errors);
    require_uuid(object, path, errors);
    match object.get("level").and_then(Value::as_i64) {
        Some(2) | Some(3) => {}
        _ => errors.push(field(&format!("{path}/level"), "invalid")),
    }
    bounded_string(object.get("text"), 1, 200, &format!("{path}/text"), errors);
    let anchor = bounded_string(object.get("anchor"), 1, 80, &format!("{path}/anchor"), errors);
    if !anchor.is_empty() && !is_anchor(&anchor) {
        errors.push(field(&format!("{path}/anchor"), "invalid"));
    }
}

fn validate_list(object: &Map<String, Value>, path: &str, errors: &mut Vec<FieldError>) {
    reject_unknown(object, &["id", "type", "ordered", "items"], path, errors);
    require_uuid(object, path, errors);
    if object.get("ordered").and_then(Value::as_bool).is_none() {
        errors.push(field(&format!("{path}/ordered"), "invalid"));
    }
    match object.get("items").and_then(Value::as_array) {
        Some(items) if (1..=50).contains(&items.len()) => {
            for (index, item) in items.iter().enumerate() {
                bounded_string(
                    Some(item),
                    1,
                    500,
                    &format!("{path}/items/{index}"),
                    errors,
                );
            }
        }
        _ => errors.push(field(&format!("{path}/items"), "invalid")),
    }
}

fn validate_quote(object: &Map<String, Value>, path: &str, errors: &mut Vec<FieldError>) {
    reject_unknown(object, &["id", "type", "text", "citation"], path, errors);
    require_uuid(object, path, errors);
    bounded_string(object.get("text"), 1, 4000, &format!("{path}/text"), errors);
    if object.contains_key("citation") {
        bounded_string(
            object.get("citation"),
            1,
            300,
            &format!("{path}/citation"),
            errors,
        );
    }
}

fn validate_table(object: &Map<String, Value>, path: &str, errors: &mut Vec<FieldError>) {
    reject_unknown(object, &["id", "type", "caption", "header", "rows"], path, errors);
    require_uuid(object, path, errors);
    if object.contains_key("caption") {
        bounded_string(
            object.get("caption"),
            1,
            200,
            &format!("{path}/caption"),
            errors,
        );
    }
    match object.get("header").and_then(Value::as_array) {
        Some(header) if (1..=12).contains(&header.len()) => {
            for (index, cell) in header.iter().enumerate() {
                bounded_string(Some(cell), 1, 80, &format!("{path}/header/{index}"), errors);
            }
        }
        _ => errors.push(field(&format!("{path}/header"), "invalid")),
    }
    match object.get("rows").and_then(Value::as_array) {
        Some(rows) if rows.len() <= 100 => {
            for (row_index, row) in rows.iter().enumerate() {
                match row.as_array() {
                    Some(cells) if (1..=12).contains(&cells.len()) => {
                        for (cell_index, cell) in cells.iter().enumerate() {
                            bounded_string(
                                Some(cell),
                                0,
                                500,
                                &format!("{path}/rows/{row_index}/{cell_index}"),
                                errors,
                            );
                        }
                    }
                    _ => errors.push(field(&format!("{path}/rows/{row_index}"), "invalid")),
                }
            }
        }
        _ => errors.push(field(&format!("{path}/rows"), "invalid")),
    }
}

fn validate_image(object: &Map<String, Value>, path: &str, errors: &mut Vec<FieldError>) {
    reject_unknown(object, &["id", "type", "alt", "url"], path, errors);
    require_uuid(object, path, errors);
    bounded_string(object.get("alt"), 1, 200, &format!("{path}/alt"), errors);
    let url = bounded_string(object.get("url"), 12, 500, &format!("{path}/url"), errors);
    if !url.is_empty() && !is_https_url(&url) {
        errors.push(field(&format!("{path}/url"), "invalid"));
    }
}

fn validate_code(object: &Map<String, Value>, path: &str, errors: &mut Vec<FieldError>) {
    reject_unknown(object, &["id", "type", "language", "text"], path, errors);
    require_uuid(object, path, errors);
    let language = bounded_string(
        object.get("language"),
        1,
        32,
        &format!("{path}/language"),
        errors,
    );
    if !language.is_empty() && !is_language(&language) {
        errors.push(field(&format!("{path}/language"), "invalid"));
    }
    bounded_string(object.get("text"), 1, 20000, &format!("{path}/text"), errors);
}

fn validate_math(object: &Map<String, Value>, path: &str, errors: &mut Vec<FieldError>) {
    reject_unknown(object, &["id", "type", "tex"], path, errors);
    require_uuid(object, path, errors);
    bounded_string(object.get("tex"), 1, 4000, &format!("{path}/tex"), errors);
}

fn validate_callout(object: &Map<String, Value>, path: &str, errors: &mut Vec<FieldError>) {
    reject_unknown(object, &["id", "type", "tone", "text"], path, errors);
    require_uuid(object, path, errors);
    match object.get("tone").and_then(Value::as_str) {
        Some("note") | Some("warning") => {}
        _ => errors.push(field(&format!("{path}/tone"), "invalid")),
    }
    bounded_string(object.get("text"), 1, 2000, &format!("{path}/text"), errors);
}

fn validate_related(object: &Map<String, Value>, path: &str, errors: &mut Vec<FieldError>) {
    reject_unknown(object, &["id", "type", "entryId"], path, errors);
    require_uuid(object, path, errors);
    match object.get("entryId").and_then(Value::as_str) {
        Some(value) if is_uuid(value) => {}
        _ => errors.push(field(&format!("{path}/entryId"), "invalid")),
    }
}

/// 개념: 글 종류에 맞는 메타데이터만 통과시킨다.
/// 목적: 로그와 일반 글의 필드가 섞이지 않게 한다.
/// 사용처: `parse_save`.
fn validate_metadata(kind: &str, metadata: &Value, errors: &mut Vec<FieldError>) {
    let Some(object) = metadata.as_object() else {
        errors.push(field("metadata", "invalid"));
        return;
    };
    if kind == "log" {
        reject_unknown(
            object,
            &["type", "projectId", "stage", "displayOrder"],
            "metadata",
            errors,
        );
        if object.get("type").and_then(Value::as_str) != Some("log") {
            errors.push(field("metadata/type", "invalid"));
        }
        match object.get("projectId").and_then(Value::as_str) {
            Some(value) if is_uuid(value) => {}
            _ => errors.push(field("metadata/projectId", "invalid")),
        }
        match object.get("stage").and_then(Value::as_str) {
            Some("planning") | Some("implementation") | Some("verification") => {}
            _ => errors.push(field("metadata/stage", "invalid")),
        }
        match object.get("displayOrder").and_then(Value::as_i64) {
            Some(value) if (0..=10_000).contains(&value) => {}
            _ => errors.push(field("metadata/displayOrder", "invalid")),
        }
        return;
    }
    reject_unknown(object, &["type"], "metadata", errors);
    let expected = match kind {
        "paper" => "paper",
        "news" => "news",
        _ => "note",
    };
    if object.get("type").and_then(Value::as_str) != Some(expected) {
        errors.push(field("metadata/type", "invalid"));
    }
}

fn require_uuid(object: &Map<String, Value>, path: &str, errors: &mut Vec<FieldError>) {
    match object.get("id").and_then(Value::as_str) {
        Some(value) if is_uuid(value) => {}
        _ => errors.push(field(&format!("{path}/id"), "invalid")),
    }
}

fn reject_unknown(
    object: &Map<String, Value>,
    allowed: &[&str],
    path: &str,
    errors: &mut Vec<FieldError>,
) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            let field_path = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}/{key}")
            };
            errors.push(field(&field_path, "unknown_field"));
        }
    }
}

fn bounded_string(
    value: Option<&Value>,
    min: usize,
    max: usize,
    path: &str,
    errors: &mut Vec<FieldError>,
) -> String {
    let Some(text) = value.and_then(Value::as_str) else {
        errors.push(field(path, "invalid"));
        return String::new();
    };
    let length = text.chars().count();
    if length < min {
        errors.push(field(path, "too_short"));
    } else if length > max {
        errors.push(field(path, "too_long"));
    }
    text.to_string()
}

fn field(path: &str, code: &str) -> FieldError {
    FieldError {
        path: path.to_string(),
        code: code.to_string(),
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
    if bytes.is_empty() || bytes.len() > 80 || bytes[0] == b'-' || *bytes.last().unwrap_or(&b'-') == b'-'
    {
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

fn is_https_url(value: &str) -> bool {
    value.starts_with("https://")
        && value.len() >= 12
        && value.len() <= 500
        && !value.chars().any(char::is_whitespace)
}

fn is_language(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'#' | b'+' | b'.' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::{parse_save, FieldError};

    fn codes(errors: &[FieldError]) -> Vec<&str> {
        errors.iter().map(|error| error.code.as_str()).collect()
    }

    #[test]
    fn paragraph_passes_and_unknown_field_fails() {
        let ok = r#"{
            "expectedVersion": 1,
            "title": "메모",
            "summary": "",
            "blocks": [{
                "id": "11111111-1111-4111-8111-111111111111",
                "type": "paragraph",
                "text": "본문"
            }],
            "metadata": {"type": "note"}
        }"#;
        assert!(parse_save(ok, "note").is_ok());

        let extra = r#"{
            "expectedVersion": 1,
            "title": "메모",
            "summary": "",
            "blocks": [{
                "id": "11111111-1111-4111-8111-111111111111",
                "type": "paragraph",
                "text": "본문",
                "html": "<script>alert(1)</script>"
            }],
            "metadata": {"type": "note"}
        }"#;
        let errors = parse_save(extra, "note").expect_err("extra field");
        assert!(codes(&errors).contains(&"unknown_field"));
    }

    #[test]
    fn unknown_block_type_fails() {
        let body = r#"{
            "expectedVersion": 1,
            "title": "메모",
            "summary": "",
            "blocks": [{"id": "11111111-1111-4111-8111-111111111111", "type": "widget"}],
            "metadata": {"type": "note"}
        }"#;
        let errors = parse_save(body, "note").expect_err("widget");
        assert!(codes(&errors).contains(&"unknown_type"));
    }
}
