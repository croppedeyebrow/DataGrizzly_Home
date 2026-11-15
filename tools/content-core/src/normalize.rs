//! 기능: 읽어 온 문서 조각을 정규화하고 절 ID를 만든다.
//! 목적: 같은 입력과 버전은 같은 결과를 내고, 한도를 넘긴 본문은 잘라 저장하지 않는다.
//! 연관: `tools/content-cli`의 `normalize -`, `workers/document-workflow`의 어댑터.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const TEXT_CHAR_LIMIT: usize = 1_000_000;

/// 기능: 추출 JSON을 정규화 결과로 바꾼다.
/// 목적: 공백과 절 ID를 한곳에서 정하고, 글자 수 한도를 넘으면 결과를 만들지 않는다.
/// 연관: `WF04`의 sections, `WF05`의 100만 자 상한.
pub fn normalize_document(input: &str) -> Result<Value, &'static str> {
    let value: Value = serde_json::from_str(input).map_err(|_| "invalid_json")?;
    let object = value.as_object().ok_or("invalid_json")?;
    if object.get("schemaVersion").and_then(Value::as_i64) != Some(1) {
        return Err("unknown_version");
    }
    let source_id = object.get("sourceId").and_then(Value::as_str).filter(|value| !value.is_empty()).ok_or("invalid_source")?;
    let source_hash = object.get("sourceHash").and_then(Value::as_str).filter(|value| !value.is_empty()).ok_or("invalid_source")?;
    let kind = object.get("kind").and_then(Value::as_str).ok_or("invalid_kind")?;
    if kind != "pdf" && kind != "markdown" {
        return Err("invalid_kind");
    }
    let parts = object.get("parts").and_then(Value::as_array).ok_or("invalid_parts")?;
    let title = object.get("title").and_then(Value::as_str).filter(|value| !value.is_empty());
    let mut warnings = Vec::new();
    if title.is_none() {
        warnings.push(json!({ "code": "missing_title" }));
    }
    let mut sections = Vec::new();
    let mut total_chars = 0usize;
    let mut saw_script = false;
    let mut saw_include = false;
    for part in parts {
        let text = normalize_text(part.get("text").and_then(Value::as_str).unwrap_or(""));
        if text.is_empty() {
            continue;
        }
        total_chars = total_chars.saturating_add(text.chars().count());
        if total_chars > TEXT_CHAR_LIMIT {
            return Err("too_many_chars");
        }
        let lowered = text.to_ascii_lowercase();
        if lowered.contains("<script") {
            saw_script = true;
        }
        if lowered.contains("!include ") || lowered.contains("{% include") {
            saw_include = true;
        }
        let page = part.get("page").and_then(Value::as_i64);
        let heading = part.get("heading").and_then(Value::as_str).unwrap_or("");
        let line_start = part.get("lineStart").and_then(Value::as_i64);
        let line_end = part.get("lineEnd").and_then(Value::as_i64);
        let section_id = section_id(source_hash, kind, sections.len(), page, heading, line_start, line_end);
        let locator = if kind == "pdf" {
            json!({ "page": page.unwrap_or(0) })
        } else {
            json!({ "heading": heading, "lineStart": line_start.unwrap_or(0), "lineEnd": line_end.unwrap_or(0) })
        };
        sections.push(json!({
            "sectionId": section_id,
            "page": page,
            "heading": if heading.is_empty() { Value::Null } else { Value::String(heading.to_string()) },
            "text": text,
            "sourceLocator": locator,
        }));
    }
    if sections.is_empty() {
        return Err("empty_text");
    }
    if saw_script {
        warnings.push(json!({ "code": "html_not_executed" }));
    }
    if saw_include {
        warnings.push(json!({ "code": "external_include_ignored" }));
    }
    Ok(json!({
        "schemaVersion": 1,
        "sourceId": source_id,
        "sourceHash": source_hash,
        "title": title,
        "authors": object.get("authors").cloned().unwrap_or_else(|| json!([])),
        "publishedAt": object.get("publishedAt").cloned().unwrap_or(Value::Null),
        "doi": object.get("doi").cloned().unwrap_or(Value::Null),
        "sections": sections,
        "warnings": warnings,
    }))
}

fn normalize_text(input: &str) -> String {
    let unified = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = Vec::new();
    for line in unified.split('\n') {
        lines.push(collapse_spaces(line.trim()));
    }
    while lines.first().is_some_and(String::is_empty) {
        lines.remove(0);
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines.join("\n")
}

fn collapse_spaces(line: &str) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    for ch in line.chars() {
        if ch == ' ' || ch == '\t' {
            pending_space = !out.is_empty();
        } else {
            if pending_space {
                out.push(' ');
            }
            pending_space = false;
            out.push(ch);
        }
    }
    out
}

fn section_id(source_hash: &str, kind: &str, ordinal: usize, page: Option<i64>, heading: &str, line_start: Option<i64>, line_end: Option<i64>) -> String {
    let material = format!(
        "{source_hash}\n{kind}\n{ordinal}\n{}\n{heading}\n{}\n{}",
        page.map(|value| value.to_string()).unwrap_or_default(),
        line_start.map(|value| value.to_string()).unwrap_or_default(),
        line_end.map(|value| value.to_string()).unwrap_or_default(),
    );
    Sha256::digest(material.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::normalize_document;

    #[test]
    fn repeated_input_keeps_the_page_and_section_id() {
        let input = r#"{"schemaVersion":1,"sourceId":"source-1","sourceHash":"abc","kind":"pdf","title":"현장","authors":[],"parts":[{"page":1,"text":"Field   note"},{"page":2,"text":"다음 장"}]}"#;
        let first = normalize_document(input).expect("first");
        let second = normalize_document(input).expect("second");
        assert_eq!(first, second);
        assert_eq!(first["sections"][0]["text"], "Field note");
        assert_eq!(first["sections"][0]["sourceLocator"]["page"], 1);
        assert_eq!(first["sections"][1]["text"], "다음 장");
        assert_eq!(first["sections"][1]["sourceLocator"]["page"], 2);
        assert_ne!(first["sections"][0]["sectionId"], first["sections"][1]["sectionId"]);
    }

    #[test]
    fn overflow_returns_no_partial_document() {
        let text = "가".repeat(1_000_001);
        let input = format!(
            r#"{{"schemaVersion":1,"sourceId":"source-1","sourceHash":"abc","kind":"markdown","parts":[{{"text":"{text}","heading":"본문","lineStart":1,"lineEnd":1}}]}}"#
        );
        assert_eq!(normalize_document(&input).expect_err("limit"), "too_many_chars");
    }

    #[test]
    fn script_stays_text_and_is_not_executed() {
        let input = r#"{"schemaVersion":1,"sourceId":"source-1","sourceHash":"abc","kind":"markdown","parts":[{"text":"<script>alert(1)</script>\n!include https://example.com/secret.md","heading":"표시","lineStart":1,"lineEnd":2}]}"#;
        let value = normalize_document(input).expect("text");
        assert!(value["sections"][0]["text"].as_str().expect("text").contains("<script>alert(1)</script>"));
        assert!(value["warnings"].to_string().contains("html_not_executed"));
        assert!(value["warnings"].to_string().contains("external_include_ignored"));
    }
}
