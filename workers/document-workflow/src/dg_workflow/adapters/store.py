"""기능: 정규화 결과를 작업별 비공개 경로에 저장한다.
목적: 원본 hash, 결과 hash, 처리 버전, 출처 위치를 남기고 다른 작업 파일은 지우지 않는다.
연관: `pipeline.py`, `workers/document-workflow/var/ingestion`.
"""

import hashlib
import json
import re
import shutil
from pathlib import Path
from typing import Any

from dg_workflow.adapters.limits import PROCESSING_VERSION
from dg_workflow.adapters.load import DocumentReadError

_JOB_ID = re.compile(r"^[0-9a-fA-F-]{36}$")


def store_artifact(root: Path, job_id: str, source_hash: str, normalized: dict[str, Any]) -> dict[str, Any]:
    """기능: 결과 JSON과 receipt를 작업 디렉터리에 쓴다.
    목적: 같은 원본 hash와 처리 버전이면 다시 쓰지 않고, 다른 결과면 기존 파일을 덮지 않는다.
    연관: `receipt.json`, `artifact.json`.
    """
    directory = job_directory(root, job_id)
    directory.mkdir(parents=True, exist_ok=True)
    body = json.dumps(normalized, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    result_hash = hashlib.sha256(body).hexdigest()
    receipt_path = directory / "receipt.json"
    if receipt_path.exists():
        stored = json.loads(receipt_path.read_text(encoding="utf-8"))
        same_input = stored.get("sourceHash") == source_hash and stored.get("processingVersion") == PROCESSING_VERSION
        if same_input and stored.get("resultHash") == result_hash:
            return stored
        raise DocumentReadError("artifact_conflict")
    artifact_path = directory / "artifact.json"
    artifact_path.write_bytes(body)
    receipt = {
        "jobId": job_id,
        "sourceHash": source_hash,
        "resultHash": result_hash,
        "processingVersion": PROCESSING_VERSION,
        "sourceLocator": _locators(normalized),
    }
    receipt_path.write_text(json.dumps(receipt, ensure_ascii=False), encoding="utf-8")
    return receipt


def remove_job_dir(root: Path, job_id: str) -> None:
    """기능: 한 작업의 중간 파일만 지운다.
    목적: 원본 fixture와 다른 작업 디렉터리는 남긴다.
    연관: `pipeline.ingest_file`의 실패 경로.
    """
    directory = job_directory(root, job_id)
    if directory.exists():
        shutil.rmtree(directory)


def job_directory(root: Path, job_id: str) -> Path:
    """기능: 작업 ID로 비공개 경로를 만든다.
    목적: 작업 ID에 경로 구분자가 들어가면 상위 디렉터리로 나가지 못하게 한다.
    연관: `remove_job_dir`.
    """
    if _JOB_ID.fullmatch(job_id) is None:
        raise DocumentReadError("invalid_job")
    return root / job_id


def _locators(normalized: dict[str, Any]) -> list[Any]:
    sections = normalized.get("sections")
    if not isinstance(sections, list):
        return []
    return [section.get("sourceLocator") for section in sections if isinstance(section, dict)]
