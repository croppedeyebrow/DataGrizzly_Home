//! 개념: 본문 검사 명령.
//! 실행: `dg-content contract`, `validate <file|->`, `extract <file>`, `chunk <file> <out>`, `export <file> <outDir>`.
//! 목적: 오류가 있으면 JSON을 stderr에 쓰고 1로 끝낸다. 그 경우 출력 파일을 바꾸지 않는다.
//! 관련: `tools/content-core`, `infra/publish/content-gate.mjs`, `apps/admin/src/editor/Editor.tsx`.

use content_core::{
    canonical_jsonl, check_document, chunk_document, exit_code, export_document, extract_sections, normalize_document, write_atomic,
    ContentError,
};
use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

/// 개념: 프로세스 진입점.
/// 실행: 인자 오류는 2, 검사 오류는 1, 성공은 0이다.
/// 목적: 라이브러리 대신 이 파일만 프로세스를 끝낸다.
/// 관련: `content_core::exit_code`.
fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::from(0),
        Err(code) => ExitCode::from(code),
    }
}

fn run(args: Vec<String>) -> Result<(), u8> {
    let command = args.first().map(String::as_str).unwrap_or("");
    // 개념: 계약 버전 출력과 표준입력 검사.
    // 실행: `contract`는 schemaVersion 1을 쓰고, `validate -`는 stdin JSON을 검사한다.
    // 목적: 워커가 파일 경로를 추정하지 않고 같은 계약을 확인하게 한다.
    // 관련: `workers/document-workflow/src/dg_workflow/rust_adapter.py`.
    if command == "contract" {
        println!(r#"{{"schemaVersion":1}}"#);
        return Ok(());
    }
    if command == "normalize" && args.get(1).map(String::as_str) == Some("-") {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input).map_err(|_| 1u8)?;
        return match normalize_document(&input) {
            Ok(value) => {
                println!("{value}");
                Ok(())
            }
            Err(code) => {
                eprintln!(r#"{{"code":"{code}"}}"#);
                Err(1)
            }
        };
    }
    if command == "validate" && args.get(1).map(String::as_str) == Some("-") {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input).map_err(|_| 1u8)?;
        return finish(&check_document(&input));
    }
    let input_path = args.get(1).ok_or_else(|| usage())?;
    let input = fs::read_to_string(input_path).map_err(|_| {
        write_errors(&[error_line("input", "invalid", "입력 파일을 읽지 못했다")]);
        1u8
    })?;
    let errors = check_document(&input);
    match command {
        "validate" => finish(&errors),
        "extract" => {
            finish(&errors)?;
            let sections: Vec<_> = extract_sections(&input)
                .into_iter()
                .map(|section| serde_json::json!({ "anchor": section.anchor, "headingPath": section.heading_path }))
                .collect();
            println!("{}", serde_json::Value::Array(sections));
            Ok(())
        }
        "chunk" => {
            finish(&errors)?;
            let out = args.get(2).ok_or_else(|| usage())?;
            let body = canonical_jsonl(&chunk_document(&input));
            write_atomic(Path::new(out), body.as_bytes()).map_err(|_| 1u8)?;
            Ok(())
        }
        "export" => {
            finish(&errors)?;
            let out = args.get(2).ok_or_else(|| usage())?;
            let files = export_document(&input, &[]).map_err(|errors| {
                write_errors(&errors);
                exit_code(&errors) as u8
            })?;
            for file in files {
                let path = Path::new(out).join(&file.name);
                write_atomic(&path, &file.bytes).map_err(|_| 1u8)?;
            }
            Ok(())
        }
        _ => Err(usage()),
    }
}

fn finish(errors: &[ContentError]) -> Result<(), u8> {
    if errors.is_empty() {
        Ok(())
    } else {
        write_errors(errors);
        Err(exit_code(errors) as u8)
    }
}

fn write_errors(errors: &[ContentError]) {
    for error in errors {
        let line = serde_json::json!({
            "path": error.path,
            "code": error.code,
            "message": error.message,
            "blockId": error.block_id,
        });
        eprintln!("{line}");
    }
}

fn error_line(path: &str, code: &str, message: &str) -> ContentError {
    ContentError { path: path.to_string(), code: code.to_string(), message: message.to_string(), block_id: None }
}

fn usage() -> u8 {
    eprintln!("usage: dg-content contract | normalize - | <validate|extract|chunk|export> <file|-> [out]");
    2
}
