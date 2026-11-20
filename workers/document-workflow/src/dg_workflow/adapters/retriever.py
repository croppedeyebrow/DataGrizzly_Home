"""기능: 공개 검색을 LangChain Retriever로 부른다.
목적: 관련 글은 Rust `GET /api/search`의 공개 결과만 쓰고, 관리자 초안 API는 부르지 않는다.
연관: `services/api/src/discovery.rs`의 `public_hit`, `graph/inspect.py`, `WF04`.
"""

from typing import Any
from urllib.parse import urlsplit

import httpx
from langchain_core.callbacks.manager import CallbackManagerForRetrieverRun
from langchain_core.documents import Document
from langchain_core.retrievers import BaseRetriever
from pydantic import ConfigDict

from dg_workflow.adapters.limits import SEARCH_TIMEOUT_SECONDS


class RelatedLookupError(Exception):
    """기능: 공개 검색을 잠시 못 읽었다는 표시.
    목적: 503과 시간 초과를 원문 추출 실패와 다른 코드로 남긴다.
    연관: `inspect_source`의 `relatedLookupFailed`.
    """

    def __init__(self) -> None:
        super().__init__("relatedLookupFailed")


class QueryRejected(Exception):
    """기능: 검색어가 공개 검색 규칙을 통과하지 못했다는 표시.
    목적: 빈 값이나 길이 거절을 서비스 장애로 적지 않는다.
    연관: `GET /api/search`의 422.
    """

    def __init__(self) -> None:
        super().__init__("query_rejected")


class SearchTargetError(Exception):
    """기능: 공개 검색 주소가 로컬 API가 아니라는 표시.
    목적: 다른 호스트나 자격 증명이 있으면 그 값을 메시지에 남기지 않는다.
    연관: `CMS_INTERNAL_BASE_URL`.
    """


def public_search_base(base_url: str) -> str:
    """기능: 검색을 보낼 로컬 출처만 고른다.
    목적: 공개 검색이 127.0.0.1 이외의 주소나 관리자 경로로 나가지 않게 한다.
    연관: `shell.require_shell_env`.
    """
    parsed = urlsplit(base_url)
    if parsed.username or parsed.password:
        raise SearchTargetError("CMS_INTERNAL_BASE_URL must not include credentials")
    if parsed.scheme != "http" or parsed.hostname != "127.0.0.1":
        raise SearchTargetError("CMS_INTERNAL_BASE_URL must be http://127.0.0.1")
    if parsed.path not in ("", "/"):
        raise SearchTargetError("CMS_INTERNAL_BASE_URL must not include a path")
    port = f":{parsed.port}" if parsed.port else ""
    return f"http://127.0.0.1{port}"


def documents_from_payload(payload: dict[str, Any]) -> list[Document]:
    """기능: 공개 검색 JSON을 Document 목록으로 바꾼다.
    목적: metadata에는 entryId, revisionId, url만 담고 snippet 이외의 본문은 넣지 않는다.
    연관: `public_hit`.
    """
    if payload.get("schemaVersion") != 1 or not isinstance(payload.get("results"), list):
        raise RelatedLookupError
    documents: list[Document] = []
    for item in payload["results"]:
        if not isinstance(item, dict):
            continue
        entry_id = item.get("entryId")
        revision_id = item.get("revisionId")
        url = item.get("url")
        title = item.get("title")
        snippet = item.get("snippet")
        if not isinstance(entry_id, str) or not entry_id:
            continue
        if not isinstance(revision_id, str) or not revision_id:
            continue
        if not isinstance(url, str) or not url.startswith("/") or url.startswith("//"):
            continue
        if not isinstance(title, str) or not title or not isinstance(snippet, str):
            continue
        documents.append(
            Document(
                page_content=snippet,
                metadata={"entryId": entry_id, "revisionId": revision_id, "url": url, "title": title},
            )
        )
    return documents


def fetch_public_documents(base_url: str, query: str, timeout: float, client: httpx.Client | None) -> list[Document]:
    """기능: `GET /api/search` 한 번을 호출한다.
    목적: 503과 시간 초과는 relatedLookupFailed로 올리고, 응답 본문을 예외 문장에 넣지 않는다.
    연관: `PublicSearchRetriever`.
    """
    if not query.strip():
        return []
    url = f"{public_search_base(base_url)}/api/search"
    owns_client = client is None
    http = client or httpx.Client(timeout=timeout)
    try:
        try:
            response = http.get(url, params={"q": query}, timeout=timeout)
        except httpx.TimeoutException as error:
            raise RelatedLookupError from error
        except httpx.HTTPError as error:
            raise RelatedLookupError from error
    finally:
        if owns_client:
            http.close()
    if response.status_code == 422:
        raise QueryRejected
    if response.status_code != 200:
        raise RelatedLookupError
    try:
        payload = response.json()
    except ValueError as error:
        raise RelatedLookupError from error
    if not isinstance(payload, dict):
        raise RelatedLookupError
    return documents_from_payload(payload)


class PublicSearchRetriever(BaseRetriever):
    """기능: 공개 검색 Retriever.
    목적: LangChain이 문자열 검색어로 공개 Document만 받게 한다.
    연관: `fetch_public_documents`, `GET /api/search`.
    """

    base_url: str
    timeout: float = SEARCH_TIMEOUT_SECONDS
    client: Any | None = None
    model_config = ConfigDict(arbitrary_types_allowed=True)

    def _get_relevant_documents(self, query: str, *, run_manager: CallbackManagerForRetrieverRun) -> list[Document]:
        _ = run_manager
        client = self.client if isinstance(self.client, httpx.Client) else None
        return fetch_public_documents(self.base_url, query, self.timeout, client)
