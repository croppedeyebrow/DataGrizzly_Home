//! 개념: 소유자 비밀번호와 세션 토큰.
//! 목적: 비밀번호는 Argon2id로만 저장하고, 세션 쿠키만으로 요청을 믿지 않는다.
//! 사용처: 서버 기동과 `/api/admin` 핸들러.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use sha2::{Digest, Sha256};

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| "password hash failed".to_string())
}

pub fn password_matches(password: &str, encoded: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(encoded) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub fn random_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    sha256_hex(&bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>())
}

pub fn sha256_hex(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn tokens_match(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (one, other) in left.iter().zip(right) {
        diff |= one ^ other;
    }
    diff == 0
}

pub fn session_cookie(token: &str) -> String {
    let secure = matches!(std::env::var("SECURE_COOKIES").as_deref(), Ok("1"));
    let mut cookie = format!(
        "dg_session={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age=43200"
    );
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

pub fn clear_session_cookie() -> String {
    let secure = matches!(std::env::var("SECURE_COOKIES").as_deref(), Ok("1"));
    let mut cookie = "dg_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0".to_string();
    if secure {
        cookie.push_str("; Secure");
    }
    cookie
}

pub fn cookie_value<'a>(header: Option<&'a str>, name: &str) -> Option<&'a str> {
    let header = header?;
    let prefix = format!("{name}=");
    header.split(';').find_map(|part| {
        let part = part.trim();
        part.strip_prefix(&prefix).filter(|value| !value.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::{hash_password, password_matches, tokens_match};

    #[test]
    fn password_roundtrip_rejects_a_different_value() {
        let encoded = hash_password("correct horse").expect("hash");
        assert!(password_matches("correct horse", &encoded));
        assert!(!password_matches("other horse", &encoded));
        assert!(!encoded.contains("correct horse"));
    }

    #[test]
    fn token_compare_checks_every_byte() {
        assert!(tokens_match("abcd", "abcd"));
        assert!(!tokens_match("abcd", "abce"));
        assert!(!tokens_match("abcd", "abc"));
    }
}
