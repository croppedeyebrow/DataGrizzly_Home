//! 개념: 릴리스 스냅샷의 수집 문서.
//! 목적: 같은 스냅샷으로 사이트맵, RSS, canonical, 슬러그 리디렉션을 맞춘다.
//! 사용처: 후보 폴더. 검색 서비스는 호출하지 않는다.

use serde_json::{json, Value};

/// 개념: 검색 색인 훅.
/// 목적: 아직 없는 검색 서비스를 성공 조건으로 부르지 않는다.
/// 사용처: 릴리스 매니페스트.
pub fn index_hook() -> &'static str {
    "skipped"
}

/// 개념: 공개 경로.
/// 목적: canonical과 사이트맵이 같은 경로를 쓰게 한다.
/// 사용처: 릴리스 피드.
pub fn canonical_path(kind: &str, slug: &str) -> String {
    let section = match kind {
        "log" => "logs",
        "paper" | "news" => "research",
        _ => "notes",
    };
    format!("/{section}/{slug}")
}

/// 개념: 수집 문서에 넣을 수 있는 주소인지.
/// 목적: 초안과 관리 주소를 사이트맵과 RSS에서 뺀다.
/// 사용처: `feed_documents`.
pub fn include_in_feed(draft: bool, path: &str) -> bool {
    !draft && path.starts_with('/') && !path.starts_with("/admin") && !path.contains("data-draft")
}

/// 개념: 슬러그가 바뀌면 이전 경로를 301로 남긴다.
/// 목적: 옛 주소와 새 canonical을 한 쌍으로 기록한다.
/// 사용처: `entry_redirect`.
pub fn redirect_for(previous: Option<&str>, next: &str) -> Option<(String, String)> {
    let previous = previous?;
    if previous.is_empty() || previous == next || !include_in_feed(false, previous) || !include_in_feed(false, next) {
        None
    } else {
        Some((previous.to_string(), next.to_string()))
    }
}

/// 개념: 사이트맵과 RSS 본문.
/// 목적: 절대 도메인을 만들지 않고, 초안은 주소 목록에서 뺀다.
/// 사용처: 후보 폴더의 sitemap.xml, rss.xml.
pub fn feed_documents(draft: bool, path: &str, title: &str) -> (String, String) {
    let loc = if include_in_feed(draft, path) { path } else { "" };
    let sitemap = if loc.is_empty() {
        "<?xml version=\"1.0\"?><urlset></urlset>".to_string()
    } else {
        format!("<?xml version=\"1.0\"?><urlset><url><loc>{loc}</loc></url></urlset>")
    };
    let rss = if loc.is_empty() {
        "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>DataGrizzly</title></channel></rss>".to_string()
    } else {
        format!(
            "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>DataGrizzly</title><item><title>{}</title><link>{loc}</link></item></channel></rss>",
            escape_xml(title)
        )
    };
    (sitemap, rss)
}

/// 개념: 스냅샷 한 건의 수집 파일.
/// 목적: 후보 폴더에 canonical과 피드를 같이 쓴다.
/// 사용처: `write_candidate`.
pub fn snapshot_feed(snapshot: &Value) -> (String, String, String, Value) {
    let draft = snapshot.get("draft").and_then(Value::as_bool).unwrap_or(false);
    let kind = snapshot.get("kind").and_then(Value::as_str).unwrap_or("note");
    let slug = snapshot.get("slug").and_then(Value::as_str).unwrap_or("");
    let title = snapshot.get("title").and_then(Value::as_str).unwrap_or("");
    let path = canonical_path(kind, slug);
    let (sitemap, rss) = feed_documents(draft, &path, title);
    let hook = json!({"called": false, "reason": index_hook()});
    (sitemap, rss, path, hook)
}

fn escape_xml(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::{canonical_path, feed_documents, include_in_feed, index_hook, redirect_for};

    /// 개념: 피드에서 초안을 빼고 슬러그 변경은 301로 남기는지.
    /// 목적: 없는 검색 서비스를 부르지 않고, 절대 도메인을 만들지 않는지 확인한다.
    /// 사용처: `cargo test`.
    #[test]
    fn feed_skips_drafts_and_records_a_slug_redirect() {
        assert_eq!(index_hook(), "skipped");
        assert!(!include_in_feed(true, "/notes/field"));
        assert!(!include_in_feed(false, "/admin/entries"));
        assert!(include_in_feed(false, "/notes/field"));
        let (sitemap, rss) = feed_documents(true, "/notes/field", "초안");
        assert!(!sitemap.contains("/notes/field"));
        assert!(!rss.contains("example.com"));
        let (live, _) = feed_documents(false, "/notes/field", "공개 <글>");
        assert!(live.contains("<loc>/notes/field</loc>"));
        assert!(!live.contains("example.com"));
        assert_eq!(canonical_path("note", "field"), "/notes/field");
        assert_eq!(
            redirect_for(Some("/notes/old"), "/notes/field"),
            Some(("/notes/old".to_string(), "/notes/field".to_string()))
        );
        assert_eq!(redirect_for(Some("/notes/field"), "/notes/field"), None);
        assert_eq!(redirect_for(Some("/admin/old"), "/notes/field"), None);
    }
}
