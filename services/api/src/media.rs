//! 개념: 첨부 파일의 이름, 형식, 저장 키 검사.
//! 목적: 원본 파일명을 경로로 쓰지 않고, 확장자만 바꾼 파일과 실행되는 SVG를 거절한다.
//! 사용처: `POST /api/admin/media`와 파일 읽기.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const MAX_BYTES: usize = 8 * 1024 * 1024;

/// 개념: 서버가 인정한 파일 종류.
/// 목적: 응답의 MIME을 파일 내용에서 정한다.
/// 사용처: 업로드 저장과 파일 응답.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Png,
    Jpeg,
    Webp,
    Pdf,
    Svg,
}

impl MediaKind {
    pub fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
            Self::Pdf => "application/pdf",
            Self::Svg => "image/svg+xml",
        }
    }
}

/// 개념: 업로드를 거절하는 이유.
/// 목적: 경로, 크기, 형식, 실행 가능한 SVG를 서로 구분한다.
/// 사용처: 422 응답의 field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectError {
    Name,
    Empty,
    TooLarge,
    Type,
    Active,
}

/// 개념: 업로드 바이트와 원본 이름을 검사한다.
/// 목적: 저장 전에 경로 탈출과 위장 파일을 막는다.
/// 사용처: `upload_media`.
pub fn inspect_upload(name: &str, bytes: &[u8]) -> Result<MediaKind, InspectError> {
    if !safe_original_name(name) {
        return Err(InspectError::Name);
    }
    if bytes.is_empty() {
        return Err(InspectError::Empty);
    }
    if bytes.len() > MAX_BYTES {
        return Err(InspectError::TooLarge);
    }
    if let Some(kind) = sniff_binary(bytes) {
        return Ok(kind);
    }
    if looks_like_svg(bytes) {
        return if svg_is_inert(bytes) {
            Ok(MediaKind::Svg)
        } else {
            Err(InspectError::Active)
        };
    }
    Err(InspectError::Type)
}

/// 개념: 화면에 보일 원본 이름.
/// 목적: 이 문자열을 디스크 경로로 해석하지 않는다.
/// 사용처: `inspect_upload`.
pub fn safe_original_name(name: &str) -> bool {
    let count = name.chars().count();
    if count == 0 || count > 180 || name.starts_with('.') {
        return false;
    }
    !name.contains('\0') && !name.contains('/') && !name.contains('\\') && !name.contains("..")
}

/// 개념: 디스크에 쓸 상대 키.
/// 목적: 요청자가 고른 이름이 `..`나 구분자로 저장 위치를 벗어나지 못하게 한다.
/// 사용처: 업로드 후 파일 읽기와 삭제.
pub fn storage_path(root: &Path, key: &str) -> Result<PathBuf, ()> {
    if key.len() != 36 || key.contains('/') || key.contains('\\') || key.contains("..") || key.contains(':') {
        return Err(());
    }
    if Uuid::parse_str(key).is_err() {
        return Err(());
    }
    let path = root.join(key);
    if path.file_name().and_then(|name| name.to_str()) != Some(key) {
        return Err(());
    }
    Ok(path)
}

/// 개념: 첨부 파일이 모이는 디렉터리.
/// 목적: 저장소 밖 경로와 `..`가 섞인 환경 값을 쓰지 않는다.
/// 사용처: 업로드와 파일 응답.
pub fn media_root() -> PathBuf {
    if let Ok(value) = std::env::var("MEDIA_ROOT") {
        if !value.is_empty() && !value.contains("..") && !value.contains('\0') {
            return PathBuf::from(value);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("var").join("media")
}

pub fn checksum(bytes: &[u8]) -> String {
    hex_encode(&Sha256::digest(bytes))
}

pub fn write_media_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}

pub fn read_media_file(path: &Path) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

pub fn remove_media_file(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// 개념: 표준 Base64를 바이트로 푼다.
/// 목적: 관리 화면이 파일을 JSON으로 보낼 때 크기 상한을 먼저 적용한다.
/// 사용처: `upload_media`.
pub fn decode_base64(input: &str) -> Result<Vec<u8>, ()> {
    fn value(byte: u8) -> Result<u8, ()> {
        match byte {
            b'A'..=b'Z' => Ok(byte - b'A'),
            b'a'..=b'z' => Ok(byte - b'a' + 26),
            b'0'..=b'9' => Ok(byte - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(()),
        }
    }
    let bytes: Vec<u8> = input.bytes().filter(|byte| !byte.is_ascii_whitespace()).collect();
    if bytes.is_empty() || bytes.len() > 12_000_000 || bytes.len() % 4 != 0 {
        return Err(());
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        if chunk[2] == b'=' && chunk[3] != b'=' {
            return Err(());
        }
        if chunk[3] == b'=' && chunk[2] != b'=' && !chunk[2].is_ascii_alphanumeric() && chunk[2] != b'+' && chunk[2] != b'/' {
            return Err(());
        }
        let n0 = value(chunk[0])?;
        let n1 = value(chunk[1])?;
        let n2 = if chunk[2] == b'=' { 0 } else { value(chunk[2])? };
        let n3 = if chunk[3] == b'=' { 0 } else { value(chunk[3])? };
        let triple = ((n0 as u32) << 18) | ((n1 as u32) << 12) | ((n2 as u32) << 6) | n3 as u32;
        out.push((triple >> 16) as u8);
        if chunk[2] != b'=' {
            out.push((triple >> 8) as u8);
        }
        if chunk[3] != b'=' {
            out.push(triple as u8);
        }
    }
    Ok(out)
}

pub fn encode_base64(input: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut index = 0;
    while index + 3 <= input.len() {
        let joined = ((input[index] as u32) << 16)
            | ((input[index + 1] as u32) << 8)
            | input[index + 2] as u32;
        out.push(TABLE[((joined >> 18) & 63) as usize] as char);
        out.push(TABLE[((joined >> 12) & 63) as usize] as char);
        out.push(TABLE[((joined >> 6) & 63) as usize] as char);
        out.push(TABLE[(joined & 63) as usize] as char);
        index += 3;
    }
    if index < input.len() {
        let remain = input.len() - index;
        let second = if remain > 1 { input[index + 1] as u32 } else { 0 };
        let joined = ((input[index] as u32) << 16) | (second << 8);
        out.push(TABLE[((joined >> 18) & 63) as usize] as char);
        out.push(TABLE[((joined >> 12) & 63) as usize] as char);
        if remain > 1 {
            out.push(TABLE[((joined >> 6) & 63) as usize] as char);
            out.push('=');
        } else {
            out.push('=');
            out.push('=');
        }
    }
    out
}

fn sniff_binary(bytes: &[u8]) -> Option<MediaKind> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(MediaKind::Png);
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(MediaKind::Jpeg);
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some(MediaKind::Webp);
    }
    if bytes.starts_with(b"%PDF-") {
        return Some(MediaKind::Pdf);
    }
    None
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let lower = text.trim_start().to_ascii_lowercase();
    lower.starts_with("<svg") || (lower.starts_with("<?xml") && lower.contains("<svg"))
}

fn svg_is_inert(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let lower = text.to_ascii_lowercase();
    const BLOCKED: &[&str] = &[
        "<script",
        "javascript:",
        "<!entity",
        "<![cdata[",
        "&#",
        "<foreignobject",
        "<iframe",
        "<embed",
        "<object",
        "<handler",
        "<style",
        "onload",
        "onerror",
        "onclick",
        "onmouseover",
        "onfocus",
        "onbegin",
        "ontoggle",
        "href=\"http",
        "href='http",
        "href=\"data",
        "href='data",
        "xlink:href",
    ];
    !BLOCKED.iter().any(|needle| lower.contains(needle))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        decode_base64, encode_base64, inspect_upload, storage_path, InspectError, MediaKind,
        MAX_BYTES,
    };
    use std::path::Path;

    fn png() -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        bytes.extend_from_slice(b"png-test");
        bytes
    }

    #[test]
    fn rejects_path_escape_and_renamed_html() {
        let png = png();
        assert_eq!(
            inspect_upload("../secret.png", &png),
            Err(InspectError::Name)
        );
        assert_eq!(
            inspect_upload(r"..\windows\secret.png", &png),
            Err(InspectError::Name)
        );
        assert_eq!(
            inspect_upload("note.png", b"<html><script>bad</script></html>"),
            Err(InspectError::Type)
        );
        assert_eq!(inspect_upload("note.png", &png), Ok(MediaKind::Png));
        assert!(storage_path(Path::new("var/media"), "../secret").is_err());
        assert!(storage_path(Path::new("var/media"), "not-a-uuid").is_err());
    }

    #[test]
    fn rejects_active_svg_and_oversized_bytes() {
        assert_eq!(
            inspect_upload("mark.svg", b"<svg><script>alert(1)</script></svg>"),
            Err(InspectError::Active)
        );
        assert_eq!(
            inspect_upload(
                "mark.svg",
                br#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#,
            ),
            Ok(MediaKind::Svg)
        );
        assert_eq!(inspect_upload("paper.pdf", b"%PDF-1.7"), Ok(MediaKind::Pdf));
        let huge = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut huge = huge;
        huge.resize(MAX_BYTES + 1, 0);
        assert_eq!(inspect_upload("big.png", &huge), Err(InspectError::TooLarge));
    }

    #[test]
    fn base64_round_trip_matches_png_prefix() {
        let encoded = encode_base64(&png());
        let decoded = decode_base64(&encoded).expect("decode");
        assert_eq!(decoded, png());
        assert!(decode_base64("@@@@").is_err());
    }
}
