//! 개념: dg-content 종료 코드.
//! 실행: 정상 파일은 0, 모르는 블록은 1이며 기존 출력은 남는다.
//! 목적: 부분 결과를 성공으로 끝내지 않는지 확인한다.
//! 관련: `tools/content-cli/src/main.rs`, `fixtures/contracts/m4-note.json`.

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

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

/// 개념: 계약 버전과 표준입력 검사.
/// 실행: `contract`는 schemaVersion 1을 출력하고, `validate -`는 JSON을 stdin으로 받는다.
/// 목적: 워커가 파일 경로를 추정하지 않고 같은 계약을 확인하게 한다.
/// 관련: `tools/content-cli/src/main.rs`, `workers/document-workflow`.
#[test]
fn contract_version_and_stdin_validate() {
    let binary = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("dg-content.exe");
    let contract = Command::new(&binary).arg("contract").output().unwrap();
    assert!(contract.status.success());
    assert_eq!(String::from_utf8(contract.stdout).unwrap().trim(), r#"{"schemaVersion":1}"#);

    let mut rejected = Command::new(&binary)
        .args(["validate", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    rejected.stdin.take().unwrap().write_all(br#"{"schemaVersion":9}"#).unwrap();
    let rejected = rejected.wait_with_output().unwrap();
    assert!(!rejected.status.success());

    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/contracts/m4-note.json");
    let mut accepted = Command::new(&binary)
        .args(["validate", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    accepted.stdin.take().unwrap().write_all(&fs::read(&fixture).unwrap()).unwrap();
    let accepted = accepted.wait_with_output().unwrap();
    assert!(accepted.status.success(), "{}", String::from_utf8_lossy(&accepted.stderr));
}
