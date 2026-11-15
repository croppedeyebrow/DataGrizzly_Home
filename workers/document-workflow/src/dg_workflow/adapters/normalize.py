"""기능: 읽은 Document를 Rust 정규화 명령으로 보낸다.
목적: 절 ID와 공백 정규화를 Python에서 다시 구현하지 않는다.
연관: `dg-content normalize -`, `rust_adapter.run_with_stdin`.
"""

import json
import subprocess
from typing import Any

from langchain_core.documents import Document

from dg_workflow.adapters.limits import EXTRACT_TIMEOUT_SECONDS
from dg_workflow.adapters.load import DocumentReadError
from dg_workflow.rust_adapter import contract_argv, run_with_stdin


def extraction_payload(documents: list[Document], source_id: str, source_hash: str) -> dict[str, Any]:
    """기능: Rust에 넘길 버전 1 추출 JSON을 만든다.
    목적: 원문 위치만 담고, 정규화된 본문과 절 ID는 비워 둔다.
    연관: `tools/content-core/src/normalize.rs`.
    """
    if not documents:
        raise DocumentReadError("empty_text")
    kind = str(documents[0].metadata.get("kind", ""))
    title = _title(documents, kind)
    parts: list[dict[str, Any]] = []
    for document in documents:
        part: dict[str, Any] = {"text": document.page_content}
        if kind == "pdf":
            part["page"] = document.metadata.get("page")
        else:
            part["heading"] = document.metadata.get("heading") or ""
            part["lineStart"] = document.metadata.get("lineStart")
            part["lineEnd"] = document.metadata.get("lineEnd")
        parts.append(part)
    return {
        "schemaVersion": 1,
        "sourceId": source_id,
        "sourceHash": source_hash,
        "kind": kind,
        "title": title,
        "authors": [],
        "publishedAt": None,
        "doi": None,
        "parts": parts,
    }


def normalize_with_rust(payload: dict[str, Any], timeout: float = EXTRACT_TIMEOUT_SECONDS) -> dict[str, Any]:
    """기능: `dg-content normalize -`에 JSON을 넘긴다.
    목적: 시간 초과와 Rust 오류 코드만 돌려주고 원문을 로그로 남기지 않는다.
    연관: `DG_CONTENT_BIN`, `contract_argv`.
    """
    argv = contract_argv()
    argv[-1:] = ["normalize", "-"]
    try:
        completed = run_with_stdin(argv, json.dumps(payload, ensure_ascii=False).encode("utf-8"), timeout)
    except subprocess.TimeoutExpired as error:
        raise DocumentReadError("timeout") from error
    if completed.returncode != 0:
        code = "normalize_failed"
        try:
            parsed = json.loads(completed.stderr.decode("utf-8"))
        except json.JSONDecodeError:
            parsed = None
        if isinstance(parsed, dict) and isinstance(parsed.get("code"), str):
            code = parsed["code"]
        raise DocumentReadError(code)
    try:
        normalized = json.loads(completed.stdout.decode("utf-8"))
    except json.JSONDecodeError as error:
        raise DocumentReadError("normalize_failed") from error
    if not isinstance(normalized, dict) or normalized.get("schemaVersion") != 1:
        raise DocumentReadError("normalize_failed")
    return normalized


def _title(documents: list[Document], kind: str) -> str | None:
    if kind != "markdown":
        return None
    heading = documents[0].metadata.get("heading")
    if isinstance(heading, str) and heading:
        return heading
    return None
