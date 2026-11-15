"""기능: 파일 읽기, Rust 정규화, 비공개 저장을 한 번에 실행한다.
목적: 실패하면 그 작업 디렉터리만 지우고 원문 fixture는 남긴다.
연관: `load.py`, `normalize.py`, `store.py`.
"""

import hashlib
from pathlib import Path
from typing import Any

from dg_workflow.adapters.limits import EXTRACT_TIMEOUT_SECONDS, MARKDOWN_BYTES, PDF_BYTES
from dg_workflow.adapters.load import DocumentReadError, load_documents
from dg_workflow.adapters.normalize import extraction_payload, normalize_with_rust
from dg_workflow.adapters.store import remove_job_dir, store_artifact


def ingest_file(
    path: Path,
    source_id: str,
    job_id: str,
    root: Path,
    timeout: float = EXTRACT_TIMEOUT_SECONDS,
) -> dict[str, Any]:
    """기능: 한 파일을 읽어 정규화 결과를 저장한다.
    목적: 한도를 넘거나 Rust가 실패하면 성공 artifact를 남기지 않는다.
    연관: `dg-content normalize -`.
    """
    try:
        _reject_size(path)
        source_hash = hashlib.sha256(path.read_bytes()).hexdigest()
        documents = load_documents(path, source_id)
        payload = extraction_payload(documents, source_id, source_hash)
        normalized = normalize_with_rust(payload, timeout)
        return store_artifact(root, job_id, source_hash, normalized)
    except DocumentReadError as error:
        if error.code != "artifact_conflict":
            remove_job_dir(root, job_id)
        raise
    except Exception:
        remove_job_dir(root, job_id)
        raise


def _reject_size(path: Path) -> None:
    if not path.is_file():
        raise DocumentReadError("missing_file")
    limit = PDF_BYTES if path.suffix.lower() == ".pdf" else MARKDOWN_BYTES
    if path.stat().st_size > limit:
        raise DocumentReadError("too_large")
