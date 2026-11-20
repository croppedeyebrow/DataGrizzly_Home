"""기능: 공개 검색 연결, 중복 분류, 검색 실패를 확인한다.
목적: 관련 글은 공개 결과만 쓰고, 같은 원문은 덮어쓰지 않으며, 검색 장애가 원문을 지우지 않는다.
연관: `dg_workflow.adapters.retriever`, `dg_workflow.graph.inspect`, `evaluations/search/m5-01.json`.
"""

import json
from pathlib import Path
from typing import Any

import httpx
import pytest

from dg_workflow.adapters.retriever import PublicSearchRetriever, SearchTargetError
from dg_workflow.graph.inspect import classify_duplicates, inspect_source, search_query

FIXTURES = Path(__file__).parent / "fixtures" / "ingestion"
JOB_ID = "33333333-3333-4333-8333-333333333333"
SOURCE_HASH = "ab" * 32
OTHER_HASH = "cd" * 32
SECRET = "비공개 원문 전체"


def _boundary() -> dict[str, Any]:
    return json.loads((FIXTURES / "public-boundary.json").read_text(encoding="utf-8"))


def _public_response() -> dict[str, Any]:
    boundary = _boundary()
    return {"schemaVersion": 1, "results": [boundary["publicResult"]]}


def _client(handler: Any) -> httpx.Client:
    return httpx.Client(transport=httpx.MockTransport(handler))


def _retriever(handler: Any) -> PublicSearchRetriever:
    return PublicSearchRetriever(base_url="http://127.0.0.1:8080", client=_client(handler))


def _hidden_texts() -> list[str]:
    return [item["text"] for item in _boundary()["hidden"]]


def test_search_query_keeps_order_and_limit() -> None:
    query = search_query(["PostgreSQL", "데이터"], ["인덱스", "postgresql"], "현장 메모")
    assert query == "PostgreSQL 데이터 인덱스 현장 메모"
    assert "?" not in query
    long = search_query([], [], "가" * 100)
    assert len(long) <= 80
    assert search_query([], [], "   ") == ""


def test_confirmed_hash_does_not_merge_or_overwrite(tmp_path: Path) -> None:
    artifact = tmp_path / JOB_ID / "artifact.json"
    artifact.parent.mkdir()
    artifact.write_text(SECRET, encoding="utf-8")
    existing = [{"id": "job-existing", "sha256": SOURCE_HASH.upper(), "title": "다른 제목"}]
    issues = classify_duplicates(SOURCE_HASH, "다른 제목", existing)
    assert issues == [{"code": "confirmed_duplicate", "existingId": "job-existing"}]
    assert "delete" not in json.dumps(issues)
    assert "merge" not in json.dumps(issues)
    payload = inspect_source(
        job_id=JOB_ID,
        source_hash=SOURCE_HASH,
        title="다른 제목",
        keywords=["메모"],
        tags=[],
        existing=[{"id": "job-existing", "sha256": SOURCE_HASH, "title": "다른 제목"}],
        extraction_code=None,
        retriever=_retriever(lambda _request: httpx.Response(200, json=_public_response())),
        root=tmp_path,
    )
    assert payload["issues"][0]["code"] == "confirmed_duplicate"
    assert artifact.read_text(encoding="utf-8") == SECRET
    assert (tmp_path / JOB_ID / "inspect.json").is_file()


def test_similar_title_is_only_a_reference() -> None:
    similar = [{"id": "entry-1", "sha256": OTHER_HASH, "title": "현장 메모"}]
    issues = classify_duplicates(SOURCE_HASH, "현장 메모 초안", similar)
    assert issues == [{"code": "reference_candidate", "existingId": "entry-1"}]
    other = [{"id": "entry-2", "sha256": OTHER_HASH, "title": "공개 노트"}]
    untouched = classify_duplicates(SOURCE_HASH, "다른 자료", other)
    assert untouched == []


def test_retriever_uses_public_search_only() -> None:
    seen: list[str] = []

    def handler(request: httpx.Request) -> httpx.Response:
        seen.append(str(request.url))
        return httpx.Response(200, json=_public_response())

    documents = _retriever(handler).invoke("현장")
    assert seen == ["http://127.0.0.1:8080/api/search?q=%ED%98%84%EC%9E%A5"]
    assert documents[0].metadata["entryId"] == "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb1"
    assert documents[0].metadata["url"] == "/notes/field-note#measure"
    assert documents[0].page_content == "현장의 데이터에서"
    encoded = json.dumps(documents[0].metadata, ensure_ascii=False) + documents[0].page_content
    for secret in _hidden_texts():
        assert secret not in encoded


def test_lookup_failure_keeps_source_and_allows_review(tmp_path: Path) -> None:
    artifact = tmp_path / JOB_ID / "artifact.json"
    artifact.parent.mkdir()
    artifact.write_text(SECRET, encoding="utf-8")

    def unavailable(_request: httpx.Request) -> httpx.Response:
        return httpx.Response(503, json={"error": "unavailable", "body": SECRET})

    payload = inspect_source(
        job_id=JOB_ID,
        source_hash=OTHER_HASH,
        title="현장 메모",
        keywords=[],
        tags=[],
        existing=[],
        extraction_code=None,
        retriever=_retriever(unavailable),
        root=tmp_path,
    )
    assert payload["warnings"] == ["relatedLookupFailed"]
    assert payload["approvable"] is True
    assert payload["reviewAllowed"] is True
    assert payload["related"] == []
    assert SECRET not in json.dumps(payload, ensure_ascii=False)
    assert artifact.read_text(encoding="utf-8") == SECRET

    def timed_out(_request: httpx.Request) -> httpx.Response:
        raise httpx.ReadTimeout(SECRET)

    timeout = inspect_source(
        job_id=JOB_ID,
        source_hash=OTHER_HASH,
        title="현장 메모",
        keywords=[],
        tags=[],
        existing=[],
        extraction_code=None,
        retriever=_retriever(timed_out),
        root=tmp_path,
    )
    assert timeout["warnings"] == ["relatedLookupFailed"]
    assert SECRET not in json.dumps(timeout, ensure_ascii=False)
    assert artifact.read_text(encoding="utf-8") == SECRET


def test_extraction_failure_is_not_a_lookup_warning() -> None:
    payload = inspect_source(
        job_id=JOB_ID,
        source_hash=OTHER_HASH,
        title="현장 메모",
        keywords=[],
        tags=[],
        existing=[],
        extraction_code="damaged",
        retriever=_retriever(lambda _request: httpx.Response(200, json=_public_response())),
    )
    assert payload["extractionCode"] == "damaged"
    assert payload["warnings"] == []
    assert payload["approvable"] is False
    assert payload["relatedEntryIds"] == ["bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb1"]
    for secret in _hidden_texts():
        assert secret not in json.dumps(payload, ensure_ascii=False)


def test_search_target_rejects_other_hosts() -> None:
    with pytest.raises(SearchTargetError) as blocked:
        PublicSearchRetriever(base_url="https://example.com").invoke("현장")
    assert "example.com" not in str(blocked.value)
    with pytest.raises(SearchTargetError) as credentials:
        PublicSearchRetriever(base_url="http://user:secret@127.0.0.1:8080").invoke("현장")
    assert "secret" not in str(credentials.value)


def test_dependencies_do_not_add_model_or_broker() -> None:
    text = (Path(__file__).parents[1] / "pyproject.toml").read_text(encoding="utf-8")
    for banned in ("openai", "faiss", "chromadb", "sentence-transformers", "redis", "celery", "pika"):
        assert banned not in text
