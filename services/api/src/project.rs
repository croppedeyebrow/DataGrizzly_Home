//! 개념: 프로젝트와 과정 기록의 입력 규칙.
//! 목적: 상태, 단계, 순서, 커밋 주소가 화면과 저장 계약을 벗어나지 않게 한다.
//! 사용처: 프로젝트 수정과 과정 기록 저장.

pub fn valid_project_status(status: &str) -> bool {
    matches!(status, "공개" | "개발 중" | "유지보수")
}

pub fn valid_stage(stage: &str) -> bool {
    let trimmed = stage.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= 40
        && !trimmed.chars().any(|item| item.is_control())
}

pub fn valid_display_order(order: i32) -> bool {
    (0..=10_000).contains(&order)
}

pub fn valid_commit_url(url: &str) -> bool {
    if url.is_empty() {
        return true;
    }
    if url.chars().count() > 300 || url.chars().any(|item| item.is_whitespace() || item.is_control()) {
        return false;
    }
    url.starts_with("https://") || url.starts_with("http://")
}

pub fn bounded_text(value: &str, max_chars: usize) -> bool {
    let count = value.chars().count();
    count > 0 && count <= max_chars && !value.chars().any(|item| item.is_control())
}

#[cfg(test)]
mod tests {
    use super::{valid_commit_url, valid_display_order, valid_project_status, valid_stage};

    #[test]
    fn project_fields_reject_blank_stage_and_odd_url() {
        assert!(valid_project_status("개발 중"));
        assert!(!valid_project_status("중단"));
        assert!(valid_stage("구현"));
        assert!(!valid_stage("  "));
        assert!(valid_display_order(0));
        assert!(!valid_display_order(-1));
        assert!(valid_commit_url(""));
        assert!(valid_commit_url("https://github.com/datagrizzly/records/commit/abc"));
        assert!(!valid_commit_url("javascript:alert(1)"));
        assert!(!valid_commit_url("https://github.com/datagrizzly/records/a b"));
    }
}
