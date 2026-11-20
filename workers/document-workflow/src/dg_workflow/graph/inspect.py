"""기능: 중복 분류와 관련 글 조회.
목적: 같은 원문은 확정 후보로, 비슷한 제목은 참고 후보로 남기고 기존 자료를 지우거나 합치지 않는다.
연관: `adapters/retriever.py`, `adapters/store.py`, `WF03`.
"""

import json
from pathlib import Path
from typing import Any

from langchain_core.documents import Document
from langchain_core.retrievers import BaseRetriever

from dg_workflow.adapters.limits import QUERY_MAX_CHARS
from dg_workflow.adapters.retriever import QueryRejected, RelatedLookupError
from dg_workflow.adapters.store import job_directory

_MIN_SIMILAR_CHARS = 4


def search_query(keywords: list[str], tags: list[str], title: str) -> str:
    """기능: 키워드, 태그, 제목을 한 검색어로 잇는다.
    목적: 질문 재작성 없이 80자 안에서 순서를 고정한다.
    연관: `discovery.rs`의 `QUERY_MAX_CHARS`.
    """
    parts: list[str] = []
    seen: set[str] = set()
    for source in (*keywords, *tags, title):
        for token in str(source).split():
            key = token.casefold()
            if not key or key in seen:
                continue
            seen.add(key)
            parts.append(token)
    text = " ".join(parts)
    if len(text) <= QUERY_MAX_CHARS:
        return text
    clipped = text[:QUERY_MAX_CHARS]
    if " " in clipped:
        clipped = clipped.rsplit(" ", 1)[0]
    return clipped.strip()


def titles_similar(left: str, right: str) -> bool:
    """기능: 제목이 참고 후보인지 본다.
    목적: 공백과 대소문자만 다르거나, 짧은 제목이 긴 제목에 그대로 있으면 참고로 올린다.
    연관: `classify_duplicates`.
    """
    first = _title_key(left)
    second = _title_key(right)
    if len(first) < 2 or len(second) < 2:
        return False
    if first == second:
        return True
    short, long = (first, second) if len(first) <= len(second) else (second, first)
    return len(short) >= _MIN_SIMILAR_CHARS and short in long


def classify_duplicates(source_hash: str, title: str, existing: list[dict[str, str]]) -> list[dict[str, str]]:
    """기능: 기존 자료와 새 원문을 비교한다.
    목적: SHA-256이 같으면 확정 중복 후보이고, 제목만 가까우면 참고 후보이다.
    연관: W03. 반환 항목에는 삭제나 병합 동작이 없다.
    """
    issues: list[dict[str, str]] = []
    current = source_hash.strip().lower()
    for item in existing:
        item_hash = str(item.get("sha256", "")).strip().lower()
        existing_id = str(item.get("id", ""))
        if current and item_hash == current:
            issues.append({"code": "confirmed_duplicate", "existingId": existing_id})
            continue
        if titles_similar(title, str(item.get("title", ""))):
            issues.append({"code": "reference_candidate", "existingId": existing_id})
    return issues


def inspect_source(
    *,
    job_id: str,
    source_hash: str,
    title: str,
    keywords: list[str],
    tags: list[str],
    existing: list[dict[str, str]],
    extraction_code: str | None,
    retriever: BaseRetriever,
    root: Path | None = None,
) -> dict[str, Any]:
    """기능: 검토에 넘길 검사 결과를 만든다.
    목적: 검색 장애는 경고만 남기고, 원문 추출 실패만 승인을 막는다.
    연관: `PublicSearchRetriever`, `inspect.json`.
    """
    warnings: list[str] = []
    related: list[dict[str, str]] = []
    query = search_query(keywords, tags, title)
    if query:
        try:
            documents = retriever.invoke(query)
        except RelatedLookupError:
            warnings.append("relatedLookupFailed")
        except QueryRejected:
            warnings.append("query_rejected")
        else:
            if isinstance(documents, list):
                related = [_related(document) for document in documents if isinstance(document, Document)]
    payload: dict[str, Any] = {
        "schemaVersion": 1,
        "jobId": job_id,
        "stage": "inspect",
        "workflowVersion": 1,
        "sourceHash": source_hash,
        "issues": classify_duplicates(source_hash, title, existing),
        "relatedEntryIds": _entry_ids(related),
        "related": related,
        "warnings": warnings,
        "extractionCode": extraction_code,
        "reviewAllowed": True,
        "approvable": extraction_code is None,
    }
    if root is not None:
        store_inspection(root, job_id, payload)
    return payload


def store_inspection(root: Path, job_id: str, payload: dict[str, Any]) -> None:
    """기능: 검사 결과만 작업 디렉터리에 쓴다.
    목적: 검색이 실패해도 원문 artifact와 receipt는 남긴다.
    연관: `store.job_directory`.
    """
    directory = job_directory(root, job_id)
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "inspect.json").write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")


def _title_key(value: str) -> str:
    return " ".join(value.split()).casefold()


def _related(document: Document) -> dict[str, str]:
    metadata = document.metadata
    return {
        "entryId": str(metadata.get("entryId", "")),
        "revisionId": str(metadata.get("revisionId", "")),
        "url": str(metadata.get("url", "")),
        "title": str(metadata.get("title", "")),
    }


def _entry_ids(related: list[dict[str, str]]) -> list[str]:
    found: list[str] = []
    for item in related:
        entry_id = item["entryId"]
        if entry_id and entry_id not in found:
            found.append(entry_id)
    return found
