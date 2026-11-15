//! 개념: dg-content 종료 코드.
//! 실행: 정상 파일은 0, 모르는 블록은 1이며 기존 출력은 남는다.
//! 목적: 부분 결과를 성공으로 끝내지 않는지 확인한다.
//! 관련: `tools/content-cli/src/main.rs`, `fixtures/contracts/m4-note.json`.

use std::fs;
use std::process::Command;

#[test]
fn invalid_document_keeps_the_previous_output() {
    let dir = std::env::temp_dir().join(format!("dg-cli-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("bad.json");
    fs::write(&bad, r#"{"schemaVersion":1,"blocks":[{"type":"widget"}]}"#).unwrap();
    let out = dir.join("chunks.jsonl");
    fs::write(&out, "kept\n").unwrap();
    let binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("dg-content.exe");
    let status = Command::new(&binary)
        .args(["chunk", bad.to_str().unwrap(), out.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(binary.exists(), "{}", binary.display());
    assert!(!status.success());
    assert_eq!(fs::read_to_string(&out).unwrap(), "kept\n");
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/contracts/m4-note.json");
    let ok = Command::new(&binary).args(["validate", fixture.to_str().unwrap()]).status().unwrap();
    assert!(ok.success());
    let _ = fs::remove_dir_all(&dir);
}
