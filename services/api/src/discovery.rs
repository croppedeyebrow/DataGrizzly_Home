//! 기능: 공개 검색 데이터와 부분 일치 규칙.
//! 목적: 발행 스냅샷의 문장만 검색 후보로 만들고, 초안과 철회 글은 결과에 넣지 않는다.
//! 연관: `migrations/0010_public_search.sql`, `snapshot.rs`의 `commit_success`, `evaluations/search/m5-01.json`.
//! 검색 서버나 언어 모델을 호출하지 않는다. 활성 릴리스 선택은 M5-02에서 이 조회에 붙인다.

use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// 기능: 한 번에 돌려주는 검색 절의 최대 개수.
/// 목적: 같은 점수일 때 결과 개수가 흔들리지 않게 한다.
/// 연관: `search_hits`, `SEARCH_SQL`.
pub const TOP_K: usize = 5;

/// 기능: 검색어의 최대 글자 수.
/// 목적: 너무 긴 입력이 조회 조건이 되지 않게 한다.
/// 연관: `prepare_query`.
pub const QUERY_MAX_CHARS: usize = 80;

/// 기능: 릴리스 하나를 적재한 뒤의 건수.
/// 목적: 후보 행과 검증한 절 수를 릴리스별로 남긴다.
/// 연관: `public_search_load`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadStats {
    pub document_count: i32,
    pub section_count: i32,
    pub validation_count: i32,
}

/// 기능: 검색에 쓰는 절 하나.
/// 목적: 제목, 본문, 분류, 앵커를 한 단위로 비교한다.
/// 연관: `sections_from_snapshot`, `search_hits`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSection {
    pub source: String,
    pub release_id: String,
    pub entry_id: String,
    pub title: String,
    pub heading: String,
    pub anchor: String,
    pub body: String,
    pub taxonomy: String,
    pub position: i32,
}

/// 기능: 검색어 검사 결과.
/// 목적: 빈 값과 너무 긴 값을 결과 없음과 구분한다.
/// 연관: `prepare_query`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    Empty,
    TooLong,
}

/// 기능: 파라미터로 넘길 부분 일치 조건.
/// 목적: 사용자가 입력한 `%`와 `_`가 모든 행을 가리키지 않게 한다.
/// 연관: `SEARCH_SQL`의 `$3`.
pub fn like_pattern(query: &str) -> String {
    let mut pattern = String::from("%");
    for character in query.chars() {
        if matches!(character, '\\' | '%' | '_') {
            pattern.push('\\');
        }
        pattern.push(character);
    }
    pattern.push('%');
    pattern
}

/// 기능: 검색어를 길이와 빈 값으로 거른다.
/// 목적: SQL 문자열에 검색어를 이어 붙이지 않고, 바인딩할 값만 만든다.
/// 연관: `like_pattern`, `search_hits`.
pub fn prepare_query(raw: &str) -> Result<String, QueryError> {
    let query = raw.trim();
    if query.is_empty() {
        return Err(QueryError::Empty);
    }
    if query.chars().count() > QUERY_MAX_CHARS {
        return Err(QueryError::TooLong);
    }
    Ok(query.to_string())
}

/// 기능: 공개 스냅샷에서 검색 절을 만든다.
/// 목적: 초안 원본이 아니라 발행 스냅샷의 문장만 후보가 되게 한다.
/// 연관: `site_release.snapshot`, `content.rs`의 블록 필드.
pub fn sections_from_snapshot(snapshot: &Value, release_id: &str, entry_id: &str, taxonomy: &str) -> Vec<SearchSection> {
    let title = snapshot.get("title").and_then(Value::as_str).unwrap_or("").to_string();
    let Some(blocks) = snapshot.get("blocks").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut sections = Vec::new();
    let mut heading = String::new();
    let mut anchor = "section".to_string();
    let mut body = String::new();
    let mut position = 0;
    for block in blocks {
        if block.get("type").and_then(Value::as_str) == Some("heading") {
            push_section(&mut sections, &title, release_id, entry_id, &heading, &anchor, &body, taxonomy, &mut position);
            heading = block.get("text").and_then(Value::as_str).unwrap_or("").to_string();
            anchor = block.get("anchor").and_then(Value::as_str).unwrap_or("section").to_string();
            body.clear();
            continue;
        }
        let piece = block_text(block);
        if piece.is_empty() {
            continue;
        }
        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(&piece);
    }
    push_section(&mut sections, &title, release_id, entry_id, &heading, &anchor, &body, taxonomy, &mut position);
    sections
}

/// 기능: 후보 절에서 검색어와 맞는 항목을 고른다.
/// 목적: 지정한 릴리스만 보고, 철회 목록과 초안은 빼며, 같은 점수는 글과 순서로 정렬한다.
/// 연관: `evaluations/search/m5-01.json`, `SEARCH_SQL`.
pub fn search_hits<'a>(sections: &'a [SearchSection], query: &str, active_release: &str, denied: &[String]) -> Vec<&'a SearchSection> {
    let needle = query.to_lowercase();
    let mut hits: Vec<&SearchSection> = sections
        .iter()
        .filter(|section| section.source == "release")
        .filter(|section| section.release_id == active_release)
        .filter(|section| !denied.iter().any(|entry| entry == &section.entry_id))
        .filter(|section| {
            let haystack = format!("{} {} {} {} {}", section.title, section.heading, section.body, section.anchor, section.taxonomy).to_lowercase();
            haystack.contains(&needle)
        })
        .collect();
    hits.sort_by(|left, right| left.entry_id.cmp(&right.entry_id).then(left.position.cmp(&right.position)).then(left.anchor.cmp(&right.anchor)));
    hits.truncate(TOP_K);
    hits
}

/// 기능: 공개 검색 SQL.
/// 목적: 검색어, 릴리스, 철회 목록, 개수 제한을 자리표시자로만 넘긴다.
/// 연관: M5-03의 `GET /api/search`. 이 태스크에서는 실행 함수만 두고 HTTP는 열지 않는다.
pub const SEARCH_SQL: &str = "SELECT d.entry_id, d.revision_id, d.release_id, d.title, s.heading, s.anchor, s.body FROM public_search_section s JOIN public_search_document d ON d.id = s.document_id WHERE d.release_id = $1 AND NOT (d.entry_id = ANY($2)) AND (d.title ILIKE $3 ESCAPE '\\' OR s.heading ILIKE $3 ESCAPE '\\' OR s.body ILIKE $3 ESCAPE '\\' OR d.taxonomy ILIKE $3 ESCAPE '\\') ORDER BY d.entry_id, s.position, s.anchor LIMIT $4";

/// 기능: 발행이 확정된 릴리스의 스냅샷을 검색 후보로 저장한다.
/// 목적: 검색 적재가 실패하면 이 트랜잭션의 발행 확정도 함께 롤백되어 이전 릴리스가 남는다.
/// 연관: `snapshot::commit_success`, `public_search_document`, `public_search_load`.
pub async fn load_release(tx: &mut Transaction<'_, Postgres>, release_id: Uuid) -> Result<LoadStats, ()> {
    let row: Option<(Uuid, Uuid, String, String, String, String)> = sqlx::query_as(
        "SELECT site_release.entry_id, site_release.revision_id, site_release.snapshot::text, entry.kind, COALESCE(project.slug, ''), COALESCE((SELECT string_agg(taxonomy.title, ' ' ORDER BY taxonomy.title) FROM entry_taxonomy JOIN taxonomy ON taxonomy.id = entry_taxonomy.taxonomy_id WHERE entry_taxonomy.entry_id = entry.id), '') FROM site_release JOIN entry ON entry.id = site_release.entry_id LEFT JOIN project ON project.id = entry.project_id WHERE site_release.id = $1",
    )
    .bind(release_id)
    .fetch_optional(tx.as_mut())
    .await
    .map_err(|_| ())?;
    let Some((entry_id, revision_id, snapshot_text, kind, project_slug, taxonomy)) = row else {
        return Err(());
    };
    let denied: Option<i32> = sqlx::query_scalar("SELECT 1 FROM entry_denylist WHERE entry_id = $1")
        .bind(entry_id)
        .fetch_optional(tx.as_mut())
        .await
        .map_err(|_| ())?;
    sqlx::query("DELETE FROM public_search_section WHERE release_id = $1")
        .bind(release_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ())?;
    sqlx::query("DELETE FROM public_search_document WHERE release_id = $1")
        .bind(release_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ())?;
    if denied.is_some() {
        let stats = LoadStats { document_count: 0, section_count: 0, validation_count: 0 };
        write_load(tx, release_id, &stats).await?;
        return Ok(stats);
    }
    let snapshot: Value = serde_json::from_str(&snapshot_text).map_err(|_| ())?;
    let sections = sections_from_snapshot(&snapshot, &release_id.to_string(), &entry_id.to_string(), &taxonomy);
    let document_id = Uuid::new_v4();
    let body = sections.iter().map(|section| section.body.as_str()).collect::<Vec<_>>().join("\n");
    let validation_count = sections.iter().filter(|section| !section.body.is_empty() || !section.heading.is_empty()).count() as i32;
    sqlx::query("INSERT INTO public_search_document (id, release_id, entry_id, revision_id, title, body, kind, project_slug, taxonomy, state) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'candidate')")
        .bind(document_id)
        .bind(release_id)
        .bind(entry_id)
        .bind(revision_id)
        .bind(snapshot.get("title").and_then(Value::as_str).unwrap_or(""))
        .bind(&body)
        .bind(&kind)
        .bind(&project_slug)
        .bind(&taxonomy)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ())?;
    for section in &sections {
        sqlx::query("INSERT INTO public_search_section (id, document_id, release_id, entry_id, revision_id, heading, anchor, body, position) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(Uuid::new_v4())
            .bind(document_id)
            .bind(release_id)
            .bind(entry_id)
            .bind(revision_id)
            .bind(&section.heading)
            .bind(&section.anchor)
            .bind(&section.body)
            .bind(section.position)
            .execute(tx.as_mut())
            .await
            .map_err(|_| ())?;
    }
    let stats = LoadStats {
        document_count: 1,
        section_count: sections.len() as i32,
        validation_count,
    };
    write_load(tx, release_id, &stats).await?;
    Ok(stats)
}

async fn write_load(tx: &mut Transaction<'_, Postgres>, release_id: Uuid, stats: &LoadStats) -> Result<(), ()> {
    sqlx::query("INSERT INTO public_search_load (release_id, document_count, section_count, validation_count) VALUES ($1, $2, $3, $4) ON CONFLICT (release_id) DO UPDATE SET document_count = EXCLUDED.document_count, section_count = EXCLUDED.section_count, validation_count = EXCLUDED.validation_count")
        .bind(release_id)
        .bind(stats.document_count)
        .bind(stats.section_count)
        .bind(stats.validation_count)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ())?;
    Ok(())
}

fn push_section(sections: &mut Vec<SearchSection>, title: &str, release_id: &str, entry_id: &str, heading: &str, anchor: &str, body: &str, taxonomy: &str, position: &mut i32) {
    if heading.is_empty() && body.is_empty() {
        return;
    }
    sections.push(SearchSection {
        source: "release".to_string(),
        release_id: release_id.to_string(),
        entry_id: entry_id.to_string(),
        title: title.to_string(),
        heading: heading.to_string(),
        anchor: anchor.to_string(),
        body: body.to_string(),
        taxonomy: taxonomy.to_string(),
        position: *position,
    });
    *position += 1;
}

fn block_text(block: &Value) -> String {
    match block.get("type").and_then(Value::as_str).unwrap_or("") {
        "paragraph" | "quote" | "callout" | "code" => block.get("text").and_then(Value::as_str).unwrap_or("").to_string(),
        "math" => block.get("tex").and_then(Value::as_str).unwrap_or("").to_string(),
        "image" => block.get("alt").and_then(Value::as_str).unwrap_or("").to_string(),
        "list" => block
            .get("items")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(|item| item.as_str()).collect::<Vec<_>>().join(" "))
            .unwrap_or_default(),
        "table" => {
            let header = block.get("header").and_then(Value::as_array).map(|values| cells(values)).unwrap_or_default();
            let rows = block
                .get("rows")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().filter_map(|row| row.as_array().map(|values| cells(values))).collect::<Vec<_>>().join(" "))
                .unwrap_or_default();
            format!("{header} {rows}").trim().to_string()
        }
        _ => String::new(),
    }
}

fn cells(values: &Vec<Value>) -> String {
    values.iter().filter_map(|value| value.as_str()).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{like_pattern, prepare_query, search_hits, sections_from_snapshot, QueryError, SearchSection, SEARCH_SQL, TOP_K};
    use serde_json::Value;

    /// 기능: 30문항과 거절 규칙.
    /// 목적: 한국어, 영문 기술어, 조사, 결과 없음, 초안, 철회, 이전 릴리스가 기대한 답과 같은지 본다.
    /// 연관: `evaluations/search/m5-01.json`.
    #[test]
    fn thirty_questions_match_the_fixture() {
        let fixture: Value = serde_json::from_str(include_str!("../../../evaluations/search/m5-01.json")).unwrap();
        let active = fixture["activeRelease"].as_str().unwrap();
        let sections = fixture["sections"].as_array().unwrap().iter().map(section_from_json).collect::<Vec<_>>();
        let questions = fixture["questions"].as_array().unwrap();
        assert_eq!(questions.len(), 30);
        for question in questions {
            let query = question["query"].as_str().unwrap();
            let denied = question.get("denied").and_then(Value::as_array).map(|items| items.iter().filter_map(|item| item.as_str().map(str::to_string)).collect::<Vec<_>>()).unwrap_or_default();
            let hits = search_hits(&sections, query, active, &denied);
            let found: Vec<&str> = hits.iter().map(|hit| hit.entry_id.as_str()).collect();
            let expect: Vec<&str> = question["expect"].as_array().unwrap().iter().filter_map(|item| item.as_str()).collect();
            assert_eq!(found, expect, "{}", question["id"]);
        }
    }

    /// 기능: 스냅샷 절 추출과 와일드카드.
    /// 목적: 제목 앵커가 유지되고, `%`가 전체 일치 문자가 되지 않는지 본다.
    /// 연관: `fixtures/contracts/m4-note.json`, `SEARCH_SQL`.
    #[test]
    fn snapshot_sections_keep_anchors_and_wildcards_stay_literal() {
        let snapshot = serde_json::json!({
            "title": "현장 메모",
            "blocks": [
                {"type": "heading", "text": "측정", "anchor": "measure"},
                {"type": "paragraph", "text": "데이터에서"},
                {"type": "code", "text": "SELECT 1"},
                {"type": "image", "alt": "표지", "url": "https://datagrizzly.local/a.png"}
            ]
        });
        let sections = sections_from_snapshot(&snapshot, "release-1", "entry-1", "데이터");
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].anchor, "measure");
        assert!(sections[0].body.contains("데이터에서"));
        assert!(sections[0].body.contains("SELECT 1"));
        assert!(sections[0].body.contains("표지"));
        assert!(!sections[0].body.contains("https://"));
        assert_eq!(prepare_query("  데이터  ").unwrap(), "데이터");
        assert_eq!(prepare_query("   "), Err(QueryError::Empty));
        assert_eq!(prepare_query(&"가".repeat(81)), Err(QueryError::TooLong));
        assert_eq!(like_pattern("%_"), "%\\%\\_%");
        assert!(SEARCH_SQL.contains("$1") && SEARCH_SQL.contains("$4"));
        assert!(!SEARCH_SQL.contains("ORDER BY $"));
        assert!(TOP_K >= 1);
    }

    fn section_from_json(value: &Value) -> SearchSection {
        SearchSection {
            source: value["source"].as_str().unwrap().to_string(),
            release_id: value["releaseId"].as_str().unwrap().to_string(),
            entry_id: value["entryId"].as_str().unwrap().to_string(),
            title: value["title"].as_str().unwrap().to_string(),
            heading: value["heading"].as_str().unwrap().to_string(),
            anchor: value["anchor"].as_str().unwrap().to_string(),
            body: value["body"].as_str().unwrap().to_string(),
            taxonomy: value["taxonomy"].as_str().unwrap().to_string(),
            position: value["position"].as_i64().unwrap() as i32,
        }
    }
}
