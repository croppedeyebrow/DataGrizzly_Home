//! 개념: 순수 검사 어댑터.
//! 실행: JSON 문자열을 받아 오류 JSON을 돌려준다. 프로세스를 끝내거나 파일을 읽지 않는다.
//! 목적: 나중에 브라우저 모듈을 만들 때도 서버와 같은 코드를 쓰게 한다.
//! 관련: `tools/content-core/src/validate.rs`, `apps/admin/src/editor/Editor.tsx`. wasm32 빌드는 이 환경에서 하지 않았다.

use content_core::check_document;
use serde_json::json;

/// 개념: 검사 결과의 JSON.
/// 실행: `check_document`의 코드와 경로만 배열로 만든다.
/// 목적: 네이티브 결과와 이후 브라우저 결과를 같은 형식으로 비교한다.
/// 관련: `fixtures/contracts/m4-note.json`.
pub fn check_json(input: &str) -> String {
    let errors: Vec<_> = check_document(input)
        .into_iter()
        .map(|error| json!({ "path": error.path, "code": error.code }))
        .collect();
    serde_json::Value::Array(errors).to_string()
}

#[cfg(test)]
mod tests {
    use super::check_json;

    /// 개념: 어댑터와 라이브러리의 코드가 같다.
    /// 실행: 고정 문서와 모르는 버전을 비교한다. 브라우저 모듈 빌드는 포함하지 않는다.
    /// 목적: 순수 함수가 저장 검사의 거절 코드를 유지하는지 본다.
    /// 관련: `tools/content-core/src/validate.rs`.
    #[test]
    fn adapter_matches_the_library_on_the_fixture() {
        let fixture = include_str!("../../../fixtures/contracts/m4-note.json");
        assert_eq!(check_json(fixture), "[]");
        let unknown = fixture.replacen("\"schemaVersion\": 1", "\"schemaVersion\": 2", 1);
        assert!(check_json(&unknown).contains("unknown_version"));
    }
}
