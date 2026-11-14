//! 개념: 첨부, 분류, 프로젝트 과정의 관리 API.
//! 목적: 소유자만 초안 파일을 보게 하고, 분류를 지워도 글과 참조 파일은 남긴다.
//! 사용처: `/api/admin/media`, `/api/admin/taxonomies`, `/api/admin/projects`, `/api/media/{id}`.

use crate::admin::{
    audit, authorize, conflict, invalid, is_unique, json_response, not_found, unavailable, valid_slug,
};
use crate::media::{self, InspectError, MediaKind};
use crate::project::{self, bounded_text};
use crate::taxonomy::{self, order_plan};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post, put};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const PAGE_SIZE: i64 = 8;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/admin/media", get(list_media).post(upload_media))
        .route("/api/admin/media/{id}", put(update_media).delete(delete_media))
        .route("/api/admin/media/{id}/file", get(admin_file))
        .route("/api/admin/media/{id}/visibility", post(set_visibility))
        .route("/api/admin/media/{id}/usages", post(attach_usage))
        .route("/api/media/{id}", get(public_file))
        .route("/api/admin/taxonomies", get(list_taxonomies).post(create_taxonomy))
        .route(
            "/api/admin/taxonomies/{id}",
            get(read_taxonomy).put(update_taxonomy).delete(delete_taxonomy),
        )
        .route("/api/admin/taxonomies/{id}/entries", put(link_entries))
        .route("/api/admin/series/{id}/order", put(reorder_series))
        .route("/api/admin/projects", get(list_projects).post(create_project))
        .route("/api/admin/projects/{id}", get(read_project).put(update_project))
        .route("/api/admin/projects/{id}/logs", post(create_log))
        .route("/api/admin/projects/{id}/logs/order", put(reorder_logs))
        .route("/api/admin/project-logs/{id}", put(update_log))
}

#[derive(Deserialize)]
struct PageQuery {
    page: Option<i32>,
}

#[derive(Deserialize)]
struct KindQuery {
    kind: Option<String>,
}

#[derive(Deserialize)]
struct UploadBody {
    filename: String,
    #[serde(rename = "contentBase64")]
    content_base64: String,
}

#[derive(Deserialize)]
struct MediaText {
    alt: String,
    caption: String,
}

#[derive(Deserialize)]
struct VisibilityBody {
    public: bool,
}

#[derive(Deserialize)]
struct UsageBody {
    #[serde(rename = "entryId")]
    entry_id: String,
}

#[derive(Deserialize)]
struct TaxonomyBody {
    kind: String,
    slug: String,
    title: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct TaxonomyUpdate {
    slug: String,
    title: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct EntryLinks {
    #[serde(rename = "entryIds")]
    entry_ids: Vec<String>,
}

#[derive(Deserialize)]
struct ProjectBody {
    slug: String,
    title: String,
    summary: String,
    status: String,
}

#[derive(Deserialize)]
struct ProjectUpdate {
    #[serde(rename = "expectedVersion")]
    expected_version: i32,
    title: String,
    summary: String,
    status: String,
}

#[derive(Deserialize)]
struct LogBody {
    slug: String,
    title: String,
    stage: String,
    #[serde(rename = "displayOrder")]
    display_order: i32,
    #[serde(rename = "commitUrl")]
    commit_url: String,
    #[serde(rename = "entryId")]
    entry_id: Option<String>,
}

#[derive(Deserialize)]
struct LogUpdate {
    #[serde(rename = "expectedVersion")]
    expected_version: i32,
    #[serde(rename = "projectId")]
    project_id: String,
    stage: String,
    #[serde(rename = "displayOrder")]
    display_order: i32,
    #[serde(rename = "commitUrl")]
    commit_url: String,
}

async fn list_media(State(pool): State<PgPool>, headers: HeaderMap, Query(query): Query<PageQuery>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let page = query.page.unwrap_or(1).clamp(1, 100);
    let offset = i64::from(page - 1) * PAGE_SIZE;
    let total: i64 = match sqlx::query_scalar("SELECT count(*) FROM media").fetch_one(&pool).await {
        Ok(total) => total,
        Err(_) => return unavailable(),
    };
    let rows = sqlx::query_as::<_, (Uuid, String, String, i32, String, bool, String, String, i64)>(
        "SELECT id, original_name, media_type, bytes, status, public, alt, caption,
                (SELECT count(*) FROM revision_media WHERE revision_media.media_id = media.id)
         FROM media
         ORDER BY created_at DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(PAGE_SIZE)
    .bind(offset)
    .fetch_all(&pool)
    .await;
    let Ok(rows) = rows else {
        return unavailable();
    };
    let media = rows
        .into_iter()
        .map(|(id, name, media_type, bytes, status, public, alt, caption, usage)| {
            json!({
                "id": id.to_string(),
                "originalName": name,
                "mediaType": media_type,
                "bytes": bytes,
                "status": status,
                "public": public,
                "alt": alt,
                "caption": caption,
                "usageCount": usage,
            })
        })
        .collect::<Vec<_>>();
    json_response(
        StatusCode::OK,
        json!({"media": media, "page": page, "pageSize": PAGE_SIZE, "total": total}).to_string(),
        None,
    )
}

async fn upload_media(State(pool): State<PgPool>, headers: HeaderMap, body: String) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Ok(input) = serde_json::from_str::<UploadBody>(&body) else {
        return invalid();
    };
    let Ok(bytes) = media::decode_base64(&input.content_base64) else {
        return field_invalid("contentBase64");
    };
    let kind = match media::inspect_upload(&input.filename, &bytes) {
        Ok(kind) => kind,
        Err(error) => return field_invalid(inspect_field(error)),
    };
    let key = Uuid::new_v4().to_string();
    let path = match media::storage_path(&media::media_root(), &key) {
        Ok(path) => path,
        Err(()) => return field_invalid("filename"),
    };
    let checksum = media::checksum(&bytes);
    let byte_count = i32::try_from(bytes.len()).unwrap_or(i32::MAX);
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let inserted: Result<Uuid, sqlx::Error> = sqlx::query_scalar(
        "INSERT INTO media (storage_key, original_name, media_type, bytes, checksum, status)
         VALUES ($1, $2, $3, $4, $5, 'uploading')
         RETURNING id",
    )
    .bind(&key)
    .bind(&input.filename)
    .bind(kind.mime())
    .bind(byte_count)
    .bind(&checksum)
    .fetch_one(tx.as_mut())
    .await;
    let id = match inserted {
        Ok(id) => id,
        Err(_) => return unavailable(),
    };
    let stored = path.clone();
    let saved = tokio::task::spawn_blocking(move || media::write_media_file(&stored, &bytes)).await;
    if !matches!(saved, Ok(Ok(()))) {
        let _ = tx.rollback().await;
        media::remove_media_file(&path);
        return unavailable();
    }
    let verified = sqlx::query("UPDATE media SET status = 'verified' WHERE id = $1")
        .bind(id)
        .execute(tx.as_mut())
        .await;
    if verified.is_err() || tx.commit().await.is_err() {
        media::remove_media_file(&path);
        return unavailable();
    }
    let _ = audit(&pool, Some(principal.owner_id), "media_uploaded").await;
    json_response(
        StatusCode::CREATED,
        json!({
            "id": id.to_string(),
            "storageKey": key,
            "mediaType": kind.mime(),
            "bytes": byte_count,
            "status": "verified",
            "public": false,
        })
        .to_string(),
        None,
    )
}

async fn update_media(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<MediaText>(&body) else {
        return invalid();
    };
    if !calm_text(&input.alt, 200) || !calm_text(&input.caption, 500) {
        return field_invalid("alt");
    }
    let updated = sqlx::query("UPDATE media SET alt = $2, caption = $3 WHERE id = $1")
        .bind(id)
        .bind(&input.alt)
        .bind(&input.caption)
        .execute(&pool)
        .await;
    match updated {
        Ok(result) if result.rows_affected() == 1 => json_response(StatusCode::OK, r#"{"ok":true}"#.to_string(), None),
        Ok(_) => not_found(),
        Err(_) => unavailable(),
    }
}

async fn delete_media(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let usage: i64 = match sqlx::query_scalar("SELECT count(*) FROM revision_media WHERE media_id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
    {
        Ok(usage) => usage,
        Err(_) => return unavailable(),
    };
    if usage > 0 {
        return referenced();
    }
    let key: Option<String> = match sqlx::query_scalar("SELECT storage_key FROM media WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await
    {
        Ok(key) => key,
        Err(_) => return unavailable(),
    };
    let Some(key) = key else {
        return not_found();
    };
    let deleted = sqlx::query("DELETE FROM media WHERE id = $1").bind(id).execute(&pool).await;
    match deleted {
        Ok(result) if result.rows_affected() == 1 => {}
        Ok(_) => return not_found(),
        Err(error) if is_foreign_key(&error) => return referenced(),
        Err(_) => return unavailable(),
    }
    if let Ok(path) = media::storage_path(&media::media_root(), &key) {
        media::remove_media_file(&path);
    }
    let _ = audit(&pool, Some(principal.owner_id), "media_deleted").await;
    json_response(StatusCode::OK, r#"{"ok":true}"#.to_string(), None)
}

async fn set_visibility(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<VisibilityBody>(&body) else {
        return invalid();
    };
    let updated = sqlx::query(
        "UPDATE media SET public = $2 WHERE id = $1 AND status IN ('verified', 'referenced')",
    )
    .bind(id)
    .bind(input.public)
    .execute(&pool)
    .await;
    match updated {
        Ok(result) if result.rows_affected() == 1 => {
            let _ = audit(&pool, Some(principal.owner_id), "media_visibility").await;
            json_response(StatusCode::OK, json!({"public": input.public}).to_string(), None)
        }
        Ok(_) => not_found(),
        Err(_) => unavailable(),
    }
}

async fn attach_usage(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(media_id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<UsageBody>(&body) else {
        return invalid();
    };
    let Some(entry_id) = parse_id(&input.entry_id) else {
        return field_invalid("entryId");
    };
    let ready: Option<String> = match sqlx::query_scalar(
        "SELECT status FROM media WHERE id = $1 AND status IN ('verified', 'referenced')",
    )
    .bind(media_id)
    .fetch_optional(&pool)
    .await
    {
        Ok(status) => status,
        Err(_) => return unavailable(),
    };
    if ready.is_none() {
        return not_found();
    }
    let revision: Option<Uuid> = match sqlx::query_scalar("SELECT draft_revision_id FROM entry WHERE id = $1")
        .bind(entry_id)
        .fetch_optional(&pool)
        .await
    {
        Ok(revision) => revision,
        Err(_) => return unavailable(),
    };
    let Some(revision) = revision else {
        return field_invalid("entryId");
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let linked = sqlx::query(
        "INSERT INTO revision_media (revision_id, media_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(revision)
    .bind(media_id)
    .execute(tx.as_mut())
    .await;
    let marked = sqlx::query("UPDATE media SET status = 'referenced' WHERE id = $1")
        .bind(media_id)
        .execute(tx.as_mut())
        .await;
    if linked.is_err() || marked.is_err() || tx.commit().await.is_err() {
        return unavailable();
    }
    json_response(StatusCode::OK, r#"{"ok":true}"#.to_string(), None)
}

async fn admin_file(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    read_stored(&pool, id, false).await
}

async fn public_file(State(pool): State<PgPool>, Path(id): Path<String>) -> Response {
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    // 철회된 글에 연결된 첨부는 공개 주소로 돌려주지 않는다. 검색 색인보다 이 조회가 먼저다.
    match crate::withdrawal::media_is_withdrawn(&pool, id).await {
        Ok(true) => return not_found(),
        Ok(false) => {}
        Err(()) => return unavailable(),
    }
    read_stored(&pool, id, true).await
}

async fn list_taxonomies(State(pool): State<PgPool>, headers: HeaderMap, Query(query): Query<KindQuery>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    if query.kind.as_deref().is_some_and(|kind| !taxonomy::valid_kind(kind)) {
        return field_invalid("kind");
    }
    let rows = sqlx::query_as::<_, (Uuid, String, String, String, String, i64)>(
        "SELECT id, kind, slug, title, description,
                (SELECT count(*) FROM entry_taxonomy WHERE entry_taxonomy.taxonomy_id = taxonomy.id)
         FROM taxonomy
         WHERE $1::text IS NULL OR kind = $1
         ORDER BY title ASC
         LIMIT 50",
    )
    .bind(query.kind.as_deref())
    .fetch_all(&pool)
    .await;
    let Ok(rows) = rows else {
        return unavailable();
    };
    let taxonomies = rows
        .into_iter()
        .map(|(id, kind, slug, title, description, entries)| {
            json!({
                "id": id.to_string(),
                "kind": kind,
                "slug": slug,
                "title": title,
                "description": description,
                "entryCount": entries,
            })
        })
        .collect::<Vec<_>>();
    json_response(StatusCode::OK, json!({"taxonomies": taxonomies}).to_string(), None)
}

async fn create_taxonomy(State(pool): State<PgPool>, headers: HeaderMap, body: String) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Ok(input) = serde_json::from_str::<TaxonomyBody>(&body) else {
        return invalid();
    };
    let description = input.description.unwrap_or_default();
    if !taxonomy::valid_kind(&input.kind)
        || !valid_slug(&input.slug)
        || !bounded_text(&input.title, 120)
        || !calm_text(&description, 500)
    {
        return field_invalid("slug");
    }
    let inserted = sqlx::query_as::<_, (Uuid,)> (
        "INSERT INTO taxonomy (kind, slug, title, description) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(&input.kind)
    .bind(&input.slug)
    .bind(&input.title)
    .bind(&description)
    .fetch_one(&pool)
    .await;
    match inserted {
        Ok((id,)) => json_response(
            StatusCode::CREATED,
            json!({"id": id.to_string(), "slug": input.slug, "kind": input.kind}).to_string(),
            None,
        ),
        Err(error) if is_unique(&error) => conflict(),
        Err(_) => unavailable(),
    }
}

async fn read_taxonomy(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let row = sqlx::query_as::<_, (Uuid, String, String, String, String)>(
        "SELECT id, kind, slug, title, description FROM taxonomy WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await;
    let Ok(row) = row else {
        return unavailable();
    };
    let Some((id, kind, slug, title, description)) = row else {
        return not_found();
    };
    let entries = if kind == "series" {
        series_entries(&pool, id).await
    } else {
        linked_entries(&pool, id).await
    };
    let Ok(entries) = entries else {
        return unavailable();
    };
    json_response(
        StatusCode::OK,
        json!({
            "id": id.to_string(),
            "kind": kind,
            "slug": slug,
            "title": title,
            "description": description,
            "entries": entries,
        })
        .to_string(),
        None,
    )
}

async fn update_taxonomy(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<TaxonomyUpdate>(&body) else {
        return invalid();
    };
    let description = input.description.unwrap_or_default();
    if !valid_slug(&input.slug) || !bounded_text(&input.title, 120) || !calm_text(&description, 500) {
        return field_invalid("slug");
    }
    let updated = sqlx::query("UPDATE taxonomy SET slug = $2, title = $3, description = $4 WHERE id = $1")
        .bind(id)
        .bind(&input.slug)
        .bind(&input.title)
        .bind(&description)
        .execute(&pool)
        .await;
    match updated {
        Ok(result) if result.rows_affected() == 1 => json_response(StatusCode::OK, r#"{"ok":true}"#.to_string(), None),
        Ok(_) => not_found(),
        Err(error) if is_unique(&error) => conflict(),
        Err(_) => unavailable(),
    }
}

async fn delete_taxonomy(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let deleted = sqlx::query("DELETE FROM taxonomy WHERE id = $1").bind(id).execute(&pool).await;
    match deleted {
        Ok(result) if result.rows_affected() == 1 => {
            let _ = audit(&pool, Some(principal.owner_id), "taxonomy_deleted").await;
            json_response(StatusCode::OK, r#"{"ok":true}"#.to_string(), None)
        }
        Ok(_) => not_found(),
        Err(_) => unavailable(),
    }
}

async fn link_entries(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<EntryLinks>(&body) else {
        return invalid();
    };
    let Some(entry_ids) = parse_ids(&input.entry_ids) else {
        return field_invalid("entryIds");
    };
    if entry_ids.len() > 50 || !taxonomy::unique_ids(&entry_ids) {
        return field_invalid("entryIds");
    }
    let kind: Option<String> = match sqlx::query_scalar("SELECT kind FROM taxonomy WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await
    {
        Ok(kind) => kind,
        Err(_) => return unavailable(),
    };
    let Some(kind) = kind else {
        return not_found();
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    for entry_id in &entry_ids {
        let exists: Option<i32> = match sqlx::query_scalar("SELECT 1 FROM entry WHERE id = $1")
            .bind(entry_id)
            .fetch_optional(tx.as_mut())
            .await
        {
            Ok(exists) => exists,
            Err(_) => return unavailable(),
        };
        if exists.is_none() {
            let _ = tx.rollback().await;
            return field_invalid("entryIds");
        }
    }
    if sqlx::query("DELETE FROM entry_taxonomy WHERE taxonomy_id = $1")
        .bind(id)
        .execute(tx.as_mut())
        .await
        .is_err()
        || sqlx::query("DELETE FROM series_entry WHERE series_id = $1")
            .bind(id)
            .execute(tx.as_mut())
            .await
            .is_err()
    {
        return unavailable();
    }
    for (index, entry_id) in entry_ids.iter().enumerate() {
        if sqlx::query("INSERT INTO entry_taxonomy (entry_id, taxonomy_id) VALUES ($1, $2)")
            .bind(entry_id)
            .bind(id)
            .execute(tx.as_mut())
            .await
            .is_err()
        {
            return unavailable();
        }
        if kind == "series" {
            let position = i32::try_from(index + 1).unwrap_or(1);
            if sqlx::query("INSERT INTO series_entry (series_id, entry_id, position) VALUES ($1, $2, $3)")
                .bind(id)
                .bind(entry_id)
                .bind(position)
                .execute(tx.as_mut())
                .await
                .is_err()
            {
                return unavailable();
            }
        }
    }
    if tx.commit().await.is_err() {
        return unavailable();
    }
    json_response(StatusCode::OK, r#"{"ok":true}"#.to_string(), None)
}

async fn reorder_series(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<EntryLinks>(&body) else {
        return invalid();
    };
    let kind: Option<String> = match sqlx::query_scalar("SELECT kind FROM taxonomy WHERE id = $1")
        .bind(id)
        .fetch_optional(&pool)
        .await
    {
        Ok(kind) => kind,
        Err(_) => return unavailable(),
    };
    if kind.as_deref() != Some("series") {
        return if kind.is_none() { not_found() } else { field_invalid("kind") };
    }
    let existing: Vec<Uuid> = match sqlx::query_scalar(
        "SELECT entry_id FROM series_entry WHERE series_id = $1 ORDER BY position ASC",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    {
        Ok(existing) => existing,
        Err(_) => return unavailable(),
    };
    let Some(entry_ids) = parse_ids(&input.entry_ids) else {
        return field_invalid("entryIds");
    };
    let Ok(plan) = order_plan(&existing, &entry_ids) else {
        return field_invalid("entryIds");
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    if sqlx::query("UPDATE series_entry SET position = position + 10000 WHERE series_id = $1")
        .bind(id)
        .execute(tx.as_mut())
        .await
        .is_err()
    {
        return unavailable();
    }
    for (entry_id, position) in plan {
        let updated = sqlx::query("UPDATE series_entry SET position = $3 WHERE series_id = $1 AND entry_id = $2")
            .bind(id)
            .bind(entry_id)
            .bind(position)
            .execute(tx.as_mut())
            .await;
        if !matches!(updated, Ok(result) if result.rows_affected() == 1) {
            let _ = tx.rollback().await;
            return unavailable();
        }
    }
    if tx.commit().await.is_err() {
        return unavailable();
    }
    json_response(StatusCode::OK, r#"{"ok":true}"#.to_string(), None)
}

async fn list_projects(State(pool): State<PgPool>, headers: HeaderMap) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let rows = sqlx::query_as::<_, (Uuid, String, String, String, String, i32, i64, String)>(
        "SELECT id, slug, title, summary, status, version,
                (SELECT count(*) FROM project_log WHERE project_log.project_id = project.id),
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
         FROM project
         ORDER BY updated_at DESC
         LIMIT 20",
    )
    .fetch_all(&pool)
    .await;
    let Ok(rows) = rows else {
        return unavailable();
    };
    let projects = rows
        .into_iter()
        .map(|(id, slug, title, summary, status, version, logs, updated_at)| {
            json!({
                "id": id.to_string(),
                "slug": slug,
                "title": title,
                "summary": summary,
                "status": status,
                "version": version,
                "logCount": logs,
                "updatedAt": updated_at,
            })
        })
        .collect::<Vec<_>>();
    json_response(StatusCode::OK, json!({"projects": projects}).to_string(), None)
}

async fn create_project(State(pool): State<PgPool>, headers: HeaderMap, body: String) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Ok(input) = serde_json::from_str::<ProjectBody>(&body) else {
        return invalid();
    };
    if !valid_project_input(&input.slug, &input.title, &input.summary, &input.status) {
        return field_invalid("slug");
    }
    let inserted = sqlx::query_as::<_, (Uuid,)>(
        "INSERT INTO project (slug, title, summary, status) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(&input.slug)
    .bind(&input.title)
    .bind(&input.summary)
    .bind(&input.status)
    .fetch_one(&pool)
    .await;
    match inserted {
        Ok((id,)) => {
            let _ = audit(&pool, Some(principal.owner_id), "project_created").await;
            json_response(
                StatusCode::CREATED,
                json!({"id": id.to_string(), "slug": input.slug, "version": 1}).to_string(),
                None,
            )
        }
        Err(error) if is_unique(&error) => conflict(),
        Err(_) => unavailable(),
    }
}

async fn read_project(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let row = sqlx::query_as::<_, (Uuid, String, String, String, String, i32, String)>(
        "SELECT id, slug, title, summary, status, version,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
         FROM project WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await;
    let row = match row {
        Ok(row) => row,
        Err(_) => return unavailable(),
    };
    let Some((id, slug, title, summary, status, version, updated_at)) = row else {
        return not_found();
    };
    let logs = match project_logs(&pool, id).await {
        Ok(logs) => logs,
        Err(()) => return unavailable(),
    };
    json_response(
        StatusCode::OK,
        json!({
            "id": id.to_string(),
            "slug": slug,
            "title": title,
            "summary": summary,
            "status": status,
            "version": version,
            "updatedAt": updated_at,
            "logs": logs,
        })
        .to_string(),
        None,
    )
}

async fn update_project(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<ProjectUpdate>(&body) else {
        return invalid();
    };
    if input.expected_version < 1 || !valid_project_input(&"", &input.title, &input.summary, &input.status) {
        return field_invalid("title");
    }
    let updated: Result<Option<i32>, sqlx::Error> = sqlx::query_scalar(
        "UPDATE project
         SET title = $2, summary = $3, status = $4, version = version + 1, updated_at = now()
         WHERE id = $1 AND version = $5
         RETURNING version",
    )
    .bind(id)
    .bind(&input.title)
    .bind(&input.summary)
    .bind(&input.status)
    .bind(input.expected_version)
    .fetch_optional(&pool)
    .await;
    match updated {
        Ok(Some(version)) => json_response(StatusCode::OK, json!({"version": version}).to_string(), None),
        Ok(None) => version_miss(&pool, "project", id).await,
        Err(_) => unavailable(),
    }
}

async fn create_log(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(project_id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<LogBody>(&body) else {
        return invalid();
    };
    if !valid_log(&input.slug, &input.title, &input.stage, input.display_order, &input.commit_url) {
        return field_invalid("stage");
    }
    let project_exists: Option<i32> = match sqlx::query_scalar("SELECT 1 FROM project WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&pool)
        .await
    {
        Ok(exists) => exists,
        Err(_) => return unavailable(),
    };
    if project_exists.is_none() {
        return not_found();
    }
    let entry_id = match input.entry_id.as_deref() {
        None => None,
        Some(value) => match parse_id(value) {
            Some(entry_id) => Some(entry_id),
            None => return field_invalid("entryId"),
        },
    };
    if let Some(entry_id) = entry_id {
        let entry_exists: Option<i32> = match sqlx::query_scalar("SELECT 1 FROM entry WHERE id = $1")
            .bind(entry_id)
            .fetch_optional(&pool)
            .await
        {
            Ok(exists) => exists,
            Err(_) => return unavailable(),
        };
        if entry_exists.is_none() {
            return field_invalid("entryId");
        }
    }
    let inserted = sqlx::query_as::<_, (Uuid, i32)>(
        "INSERT INTO project_log (project_id, entry_id, slug, title, stage, display_order, commit_url)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, version",
    )
    .bind(project_id)
    .bind(entry_id)
    .bind(&input.slug)
    .bind(&input.title)
    .bind(input.stage.trim())
    .bind(input.display_order)
    .bind(&input.commit_url)
    .fetch_one(&pool)
    .await;
    match inserted {
        Ok((id, version)) => json_response(
            StatusCode::CREATED,
            json!({"id": id.to_string(), "version": version}).to_string(),
            None,
        ),
        Err(error) if is_unique(&error) => conflict(),
        Err(_) => unavailable(),
    }
}

async fn update_log(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    let principal = match authorize(&pool, &headers).await {
        Ok(principal) => principal,
        Err(response) => return response,
    };
    let Some(id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<LogUpdate>(&body) else {
        return invalid();
    };
    if input.expected_version < 1
        || !project::valid_stage(&input.stage)
        || !project::valid_display_order(input.display_order)
        || !project::valid_commit_url(&input.commit_url)
    {
        return field_invalid("stage");
    }
    let Some(project_id) = parse_id(&input.project_id) else {
        return field_invalid("projectId");
    };
    let project_exists: Option<i32> = match sqlx::query_scalar("SELECT 1 FROM project WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&pool)
        .await
    {
        Ok(exists) => exists,
        Err(_) => return unavailable(),
    };
    if project_exists.is_none() {
        return field_invalid("projectId");
    }
    let updated: Result<Option<i32>, sqlx::Error> = sqlx::query_scalar(
        "UPDATE project_log
         SET project_id = $2, stage = $3, display_order = $4, commit_url = $5, version = version + 1
         WHERE id = $1 AND version = $6
         RETURNING version",
    )
    .bind(id)
    .bind(project_id)
    .bind(input.stage.trim())
    .bind(input.display_order)
    .bind(&input.commit_url)
    .bind(input.expected_version)
    .fetch_optional(&pool)
    .await;
    match updated {
        Ok(Some(version)) => {
            let _ = audit(&pool, Some(principal.owner_id), "project_log_updated").await;
            json_response(StatusCode::OK, json!({"version": version}).to_string(), None)
        }
        Ok(None) => version_miss(&pool, "project_log", id).await,
        Err(_) => unavailable(),
    }
}

async fn reorder_logs(State(pool): State<PgPool>, headers: HeaderMap, Path(id): Path<String>, body: String) -> Response {
    if let Err(response) = authorize(&pool, &headers).await {
        return response;
    }
    let Some(project_id) = parse_id(&id) else {
        return not_found();
    };
    let Ok(input) = serde_json::from_str::<EntryLinks>(&body) else {
        return invalid();
    };
    let project_exists: Option<i32> = match sqlx::query_scalar("SELECT 1 FROM project WHERE id = $1")
        .bind(project_id)
        .fetch_optional(&pool)
        .await
    {
        Ok(exists) => exists,
        Err(_) => return unavailable(),
    };
    if project_exists.is_none() {
        return not_found();
    }
    let existing: Vec<Uuid> = match sqlx::query_scalar(
        "SELECT id FROM project_log WHERE project_id = $1 ORDER BY display_order ASC, created_at ASC, id ASC",
    )
    .bind(project_id)
    .fetch_all(&pool)
    .await
    {
        Ok(existing) => existing,
        Err(_) => return unavailable(),
    };
    let Some(entry_ids) = parse_ids(&input.entry_ids) else {
        return field_invalid("entryIds");
    };
    let Ok(plan) = order_plan(&existing, &entry_ids) else {
        return field_invalid("entryIds");
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    for (log_id, position) in plan {
        let updated = sqlx::query(
            "UPDATE project_log SET display_order = $2 WHERE id = $1 AND project_id = $3",
        )
        .bind(log_id)
        .bind(position)
        .bind(project_id)
        .execute(tx.as_mut())
        .await;
        if !matches!(updated, Ok(result) if result.rows_affected() == 1) {
            let _ = tx.rollback().await;
            return unavailable();
        }
    }
    if tx.commit().await.is_err() {
        return unavailable();
    }
    json_response(StatusCode::OK, r#"{"ok":true}"#.to_string(), None)
}

async fn read_stored(pool: &PgPool, id: Uuid, public_only: bool) -> Response {
    let row = sqlx::query_as::<_, (String, String, bool, String)>(
        "SELECT storage_key, media_type, public, status FROM media WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await;
    let Ok(row) = row else {
        return unavailable();
    };
    let Some((key, media_type, public, status)) = row else {
        return not_found();
    };
    if status != "verified" && status != "referenced" {
        return not_found();
    }
    if public_only && !public {
        return not_found();
    }
    let Ok(path) = media::storage_path(&media::media_root(), &key) else {
        return not_found();
    };
    let loaded = tokio::task::spawn_blocking(move || media::read_media_file(&path)).await;
    let Ok(Ok(bytes)) = loaded else {
        return unavailable();
    };
    file_response(&media_type, bytes)
}

async fn series_entries(pool: &PgPool, id: Uuid) -> Result<Vec<Value>, ()> {
    let rows = sqlx::query_as::<_, (Uuid, String, i32)>(
        "SELECT entry.id, entry.title, series_entry.position
         FROM series_entry
         JOIN entry ON entry.id = series_entry.entry_id
         WHERE series_entry.series_id = $1
         ORDER BY series_entry.position ASC",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|_| ())?;
    Ok(rows
        .into_iter()
        .map(|(id, title, position)| json!({"id": id.to_string(), "title": title, "position": position}))
        .collect())
}

async fn linked_entries(pool: &PgPool, id: Uuid) -> Result<Vec<Value>, ()> {
    let rows = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT entry.id, entry.title
         FROM entry_taxonomy
         JOIN entry ON entry.id = entry_taxonomy.entry_id
         WHERE entry_taxonomy.taxonomy_id = $1
         ORDER BY entry.title ASC",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|_| ())?;
    Ok(rows
        .into_iter()
        .map(|(id, title)| json!({"id": id.to_string(), "title": title}))
        .collect())
}

async fn project_logs(pool: &PgPool, id: Uuid) -> Result<Vec<Value>, ()> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, String, i32, String, Option<Uuid>, i32)>(
        "SELECT id, slug, title, stage, display_order, commit_url, entry_id, version
         FROM project_log
         WHERE project_id = $1
         ORDER BY display_order ASC, created_at ASC, id ASC",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|_| ())?;
    Ok(rows
        .into_iter()
        .map(|(id, slug, title, stage, display_order, commit_url, entry_id, version)| {
            json!({
                "id": id.to_string(),
                "slug": slug,
                "title": title,
                "stage": stage,
                "displayOrder": display_order,
                "commitUrl": commit_url,
                "entryId": entry_id.map(|value| value.to_string()),
                "version": version,
            })
        })
        .collect())
}

async fn version_miss(pool: &PgPool, table: &str, id: Uuid) -> Response {
    let exists = match table {
        "project" => sqlx::query_scalar::<_, i32>("SELECT 1 FROM project WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten(),
        "project_log" => sqlx::query_scalar::<_, i32>("SELECT 1 FROM project_log WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten(),
        _ => None,
    };
    if exists.is_some() {
        conflict()
    } else {
        not_found()
    }
}

fn valid_project_input(slug: &str, title: &str, summary: &str, status: &str) -> bool {
    (slug.is_empty() || valid_slug(slug))
        && bounded_text(title, 120)
        && calm_text(summary, 500)
        && project::valid_project_status(status)
}

fn valid_log(slug: &str, title: &str, stage: &str, display_order: i32, commit_url: &str) -> bool {
    valid_slug(slug)
        && bounded_text(title, 120)
        && project::valid_stage(stage)
        && project::valid_display_order(display_order)
        && project::valid_commit_url(commit_url)
}

fn calm_text(value: &str, max_chars: usize) -> bool {
    value.chars().count() <= max_chars && !value.chars().any(|item| item.is_control())
}

fn inspect_field(error: InspectError) -> &'static str {
    match error {
        InspectError::Name => "filename",
        InspectError::Empty | InspectError::TooLarge => "bytes",
        InspectError::Type | InspectError::Active => "mediaType",
    }
}

fn parse_id(value: &str) -> Option<Uuid> {
    Uuid::parse_str(value).ok()
}

fn parse_ids(values: &[String]) -> Option<Vec<Uuid>> {
    let mut ids = Vec::with_capacity(values.len());
    for value in values {
        ids.push(parse_id(value)?);
    }
    Some(ids)
}

fn is_foreign_key(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|database| database.code())
        .is_some_and(|code| code == "23503")
}

fn field_invalid(field: &str) -> Response {
    json_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({"error": "invalid", "field": field}).to_string(),
        None,
    )
}

fn referenced() -> Response {
    json_response(StatusCode::CONFLICT, r#"{"error":"referenced"}"#.to_string(), None)
}

fn file_response(media_type: &str, bytes: Vec<u8>) -> Response {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", media_type)
        .header("x-content-type-options", "nosniff")
        .header("cache-control", "private, no-store");
    if media_type == MediaKind::Svg.mime() {
        builder = builder.header("content-security-policy", "default-src 'none'; sandbox");
    }
    builder.body(Body::from(bytes)).expect("file body")
}

#[cfg(test)]
mod tests {
    use super::PAGE_SIZE;
    use crate::auth::{hash_password, random_token};
    use crate::media::encode_base64;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;
    use uuid::Uuid;

    #[test]
    fn media_migration_keeps_referenced_files() {
        let text = include_str!("../migrations/0004_media_taxonomy.sql");
        assert!(text.contains("REFERENCES media (id) ON DELETE RESTRICT"));
        assert!(text.contains("REFERENCES revision (id) ON DELETE RESTRICT"));
        assert!(text.contains("REFERENCES taxonomy (id) ON DELETE CASCADE"));
        assert!(!text.to_lowercase().contains("drop table"));
        assert_eq!(PAGE_SIZE, 8);
    }

    #[tokio::test]
    #[ignore = "needs DB_URL, DB_USER, DB_PASSWORD and datagrizzly_m2"]
    async fn media_taxonomy_and_project_links_keep_entries() {
        let pool = crate::db::connect_database("datagrizzly_m2")
            .await
            .expect("isolated database");
        crate::db::migrate(&pool).await.expect("migrate");
        for table in [
            "site_release",
            "publication_upload",
            "publication_request",
            "publication_schedule",
            "publication_job",
            "entry_redirect",
            "withdrawal",
            "entry_denylist",
            "revision_media",
            "series_entry",
            "entry_taxonomy",
            "media",
            "taxonomy",
            "project_log",
            "revision",
            "entry",
            "project",
            "audit_event",
            "admin_session",
            "owner",
        ] {
            sqlx::query(&format!("DELETE FROM {table}"))
                .execute(&pool)
                .await
                .expect("clear isolated rows");
        }

        let password = random_token();
        sqlx::query("INSERT INTO owner (password_hash) VALUES ($1)")
            .bind(hash_password(&password).expect("hash"))
            .execute(&pool)
            .await
            .expect("owner");
        let logged_in = call(
            &pool,
            "POST",
            "/api/admin/session",
            None,
            None,
            &format!(r#"{{"password":"{password}"}}"#),
        )
        .await;
        assert_eq!(logged_in.0, StatusCode::OK);
        let session = logged_in.2.expect("cookie").split(';').next().expect("pair").to_string();
        let csrf = json_of(&logged_in.3)["csrfToken"].as_str().expect("csrf").to_string();

        let png = {
            let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
            bytes.extend_from_slice(b"png-test");
            bytes
        };
        let escaped = call(
            &pool,
            "POST",
            "/api/admin/media",
            Some(&session),
            Some(&csrf),
            &serde_json::json!({"filename": "../secret.png", "contentBase64": encode_base64(&png)}).to_string(),
        )
        .await;
        assert_eq!(escaped.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(json_of(&escaped.3)["field"], "filename");

        let renamed = call(
            &pool,
            "POST",
            "/api/admin/media",
            Some(&session),
            Some(&csrf),
            &serde_json::json!({
                "filename": "note.png",
                "contentBase64": encode_base64(b"<html><script>bad</script></html>")
            })
            .to_string(),
        )
        .await;
        assert_eq!(renamed.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(json_of(&renamed.3)["field"], "mediaType");

        let active = call(
            &pool,
            "POST",
            "/api/admin/media",
            Some(&session),
            Some(&csrf),
            &serde_json::json!({
                "filename": "mark.svg",
                "contentBase64": encode_base64(b"<svg><script>alert(1)</script></svg>")
            })
            .to_string(),
        )
        .await;
        assert_eq!(active.0, StatusCode::UNPROCESSABLE_ENTITY);

        let uploaded = call(
            &pool,
            "POST",
            "/api/admin/media",
            Some(&session),
            Some(&csrf),
            &serde_json::json!({"filename": "note.png", "contentBase64": encode_base64(&png)}).to_string(),
        )
        .await;
        assert_eq!(uploaded.0, StatusCode::CREATED);
        let uploaded_json = json_of(&uploaded.3);
        let media_id = uploaded_json["id"].as_str().expect("media id");
        let storage_key = uploaded_json["storageKey"].as_str().expect("key");
        assert_ne!(storage_key, "note.png");
        assert!(Uuid::parse_str(storage_key).is_ok());
        assert!(!storage_key.contains(".."));

        let hidden = call(&pool, "GET", &format!("/api/media/{media_id}"), None, None, "").await;
        assert_eq!(hidden.0, StatusCode::NOT_FOUND);
        assert_ne!(&hidden.3[..hidden.3.len().min(8)], &png[..8]);

        let private_file = call(
            &pool,
            "GET",
            &format!("/api/admin/media/{media_id}/file"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(private_file.0, StatusCode::OK);
        assert_eq!(private_file.1.as_deref(), Some("image/png"));
        assert_eq!(&private_file.3[..8], &png[..8]);

        let published = call(
            &pool,
            "POST",
            &format!("/api/admin/media/{media_id}/visibility"),
            Some(&session),
            Some(&csrf),
            r#"{"public":true}"#,
        )
        .await;
        assert_eq!(published.0, StatusCode::OK);
        let visible = call(&pool, "GET", &format!("/api/media/{media_id}"), None, None, "").await;
        assert_eq!(visible.0, StatusCode::OK);
        let withdrawn = call(
            &pool,
            "POST",
            &format!("/api/admin/media/{media_id}/visibility"),
            Some(&session),
            Some(&csrf),
            r#"{"public":false}"#,
        )
        .await;
        assert_eq!(withdrawn.0, StatusCode::OK);
        let hidden_again = call(&pool, "GET", &format!("/api/media/{media_id}"), None, None, "").await;
        assert_eq!(hidden_again.0, StatusCode::NOT_FOUND);

        let created = call(
            &pool,
            "POST",
            "/api/admin/entries",
            Some(&session),
            Some(&csrf),
            r#"{"slug":"field-note","title":"현장 메모","kind":"note"}"#,
        )
        .await;
        assert_eq!(created.0, StatusCode::CREATED);
        let entry_id = json_of(&created.3)["id"].as_str().expect("entry").to_string();
        let second = call(
            &pool,
            "POST",
            "/api/admin/entries",
            Some(&session),
            Some(&csrf),
            r#"{"slug":"field-note-2","title":"다음 메모","kind":"note"}"#,
        )
        .await;
        let second_id = json_of(&second.3)["id"].as_str().expect("second").to_string();
        let linked = call(
            &pool,
            "POST",
            &format!("/api/admin/media/{media_id}/usages"),
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"entryId":"{entry_id}"}}"#),
        )
        .await;
        assert_eq!(linked.0, StatusCode::OK);
        let kept = call(
            &pool,
            "DELETE",
            &format!("/api/admin/media/{media_id}"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(kept.0, StatusCode::CONFLICT);
        assert_eq!(json_of(&kept.3)["error"], "referenced");
        let still_there = call(
            &pool,
            "GET",
            &format!("/api/admin/media/{media_id}/file"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(still_there.0, StatusCode::OK);

        let topic = call(
            &pool,
            "POST",
            "/api/admin/taxonomies",
            Some(&session),
            Some(&csrf),
            r#"{"kind":"topic","slug":"field","title":"현장"}"#,
        )
        .await;
        assert_eq!(topic.0, StatusCode::CREATED);
        let topic_id = json_of(&topic.3)["id"].as_str().expect("topic").to_string();
        let topic_link = call(
            &pool,
            "PUT",
            &format!("/api/admin/taxonomies/{topic_id}/entries"),
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"entryIds":["{entry_id}"]}}"#),
        )
        .await;
        assert_eq!(topic_link.0, StatusCode::OK);
        let removed = call(
            &pool,
            "DELETE",
            &format!("/api/admin/taxonomies/{topic_id}"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(removed.0, StatusCode::OK);
        let entry_left = call(
            &pool,
            "GET",
            &format!("/api/admin/entries/{entry_id}"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(entry_left.0, StatusCode::OK);
        assert!(json_of(&entry_left.3)["title"].as_str().unwrap().contains("현장 메모"));

        let series = call(
            &pool,
            "POST",
            "/api/admin/taxonomies",
            Some(&session),
            Some(&csrf),
            r#"{"kind":"series","slug":"notes","title":"노트 순서"}"#,
        )
        .await;
        let series_id = json_of(&series.3)["id"].as_str().expect("series").to_string();
        let series_link = call(
            &pool,
            "PUT",
            &format!("/api/admin/taxonomies/{series_id}/entries"),
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"entryIds":["{entry_id}","{second_id}"]}}"#),
        )
        .await;
        assert_eq!(series_link.0, StatusCode::OK);
        let swapped = call(
            &pool,
            "PUT",
            &format!("/api/admin/series/{series_id}/order"),
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"entryIds":["{second_id}","{entry_id}"]}}"#),
        )
        .await;
        assert_eq!(swapped.0, StatusCode::OK);
        let ordered = call(
            &pool,
            "GET",
            &format!("/api/admin/taxonomies/{series_id}"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        let ordered_json = json_of(&ordered.3);
        let entries = ordered_json["entries"].as_array().expect("entries");
        assert_eq!(entries[0]["id"], second_id);
        assert_eq!(entries[0]["position"], 1);
        assert_eq!(entries[1]["id"], entry_id);
        let duplicate = call(
            &pool,
            "PUT",
            &format!("/api/admin/series/{series_id}/order"),
            Some(&session),
            Some(&csrf),
            &format!(r#"{{"entryIds":["{entry_id}","{entry_id}"]}}"#),
        )
        .await;
        assert_eq!(duplicate.0, StatusCode::UNPROCESSABLE_ENTITY);
        let unchanged = call(
            &pool,
            "GET",
            &format!("/api/admin/taxonomies/{series_id}"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        assert_eq!(json_of(&unchanged.3)["entries"][0]["id"], second_id);

        let project = call(
            &pool,
            "POST",
            "/api/admin/projects",
            Some(&session),
            Some(&csrf),
            r#"{"slug":"memo-desk","title":"메모 책상","summary":"","status":"개발 중"}"#,
        )
        .await;
        assert_eq!(project.0, StatusCode::CREATED);
        let project_id = json_of(&project.3)["id"].as_str().expect("project").to_string();
        let log = call(
            &pool,
            "POST",
            &format!("/api/admin/projects/{project_id}/logs"),
            Some(&session),
            Some(&csrf),
            r#"{"slug":"first-step","title":"첫 기록","stage":"구현","displayOrder":1,"commitUrl":""}"#,
        )
        .await;
        assert_eq!(log.0, StatusCode::CREATED);
        let log_id = json_of(&log.3)["id"].as_str().expect("log").to_string();
        let missing = call(
            &pool,
            "PUT",
            &format!("/api/admin/project-logs/{log_id}"),
            Some(&session),
            Some(&csrf),
            r#"{"expectedVersion":1,"projectId":"00000000-0000-0000-0000-000000000000","stage":"구현","displayOrder":1,"commitUrl":""}"#,
        )
        .await;
        assert_eq!(missing.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(json_of(&missing.3)["field"], "projectId");
        let moved = call(
            &pool,
            "PUT",
            &format!("/api/admin/project-logs/{log_id}"),
            Some(&session),
            Some(&csrf),
            &format!(
                r#"{{"expectedVersion":1,"projectId":"{project_id}","stage":"검증","displayOrder":2,"commitUrl":"https://github.com/datagrizzly/records/commit/abc"}}"#
            ),
        )
        .await;
        assert_eq!(moved.0, StatusCode::OK);
        let project_body = call(
            &pool,
            "GET",
            &format!("/api/admin/projects/{project_id}"),
            Some(&session),
            Some(&csrf),
            "",
        )
        .await;
        let saved_log = &json_of(&project_body.3)["logs"][0];
        assert_eq!(saved_log["stage"], "검증");
        assert_eq!(saved_log["displayOrder"], 2);
        assert_eq!(saved_log["id"], log_id);
    }

    async fn call(
        pool: &sqlx::PgPool,
        method: &str,
        uri: &str,
        cookie: Option<&str>,
        csrf: Option<&str>,
        body: &str,
    ) -> (StatusCode, Option<String>, Option<String>, Vec<u8>) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(cookie) = cookie {
            builder = builder.header("cookie", cookie);
        }
        if let Some(csrf) = csrf {
            builder = builder.header("x-csrf-token", csrf);
        }
        let response = crate::app(pool.clone())
            .oneshot(builder.body(Body::from(body.to_string())).expect("request"))
            .await
            .expect("response");
        let status = response.status();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let set_cookie = response
            .headers()
            .get("set-cookie")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let bytes = response.into_body().collect().await.expect("body").to_bytes().to_vec();
        (status, content_type, set_cookie, bytes)
    }

    fn json_of(bytes: &[u8]) -> Value {
        serde_json::from_slice(bytes).expect("json")
    }
}
