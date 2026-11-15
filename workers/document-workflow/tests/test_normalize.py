"""기능: 문서 읽기, Rust 정규화, 실패 파일 정리를 확인한다.
목적: 같은 fixture는 같은 절 ID를 내고, 손상·한도 초과는 성공으로 저장되지 않는다.
연관: `dg_workflow.adapters`, `tests/fixtures/ingestion`.
"""

import os
import subprocess
import uuid
from pathlib import Path

import pytest
from pypdf import PdfWriter

from dg_workflow.adapters.load import DocumentReadError, load_documents
from dg_workflow.adapters.pipeline import ingest_file

FIXTURES = Path(__file__).parent / "fixtures" / "ingestion"
JOB_ID = "11111111-1111-4111-8111-111111111111"
OTHER_JOB = "22222222-2222-4222-8222-222222222222"


def test_pdf_loader_keeps_raw_pages() -> None:
    documents = load_documents(FIXTURES / "normal.pdf", "source-1")
    assert [document.metadata["page"] for document in documents] == [1, 2]
    assert "Field   note" in documents[0].page_content
    assert "Second page" in documents[1].page_content


def test_markdown_loader_keeps_heading_and_lines() -> None:
    documents = load_documents(FIXTURES / "korean.md", "source-1")
    assert documents[0].metadata["heading"] == "현장 메모"
    assert documents[0].metadata["lineStart"] == 1
    assert "첫 문단입니다." in documents[0].page_content
    assert documents[1].metadata["heading"] == "세부"
    assert "둘째 문단입니다." in documents[1].page_content


def test_missing_title_and_script_stay_text() -> None:
    missing = load_documents(FIXTURES / "missing-meta.md", "source-1")
    assert missing[0].metadata["heading"] == ""
    assert "본문만 있는 메모" in missing[0].page_content
    script = load_documents(FIXTURES / "script.md", "source-1")
    assert "<script>alert(1)</script>" in script[0].page_content
    assert "!include https://example.com/secret.md" in script[0].page_content


def test_image_and_damaged_pdfs_stop() -> None:
    with pytest.raises(DocumentReadError, match="image_only"):
        load_documents(FIXTURES / "image-only.pdf", "source-1")
    with pytest.raises(DocumentReadError, match="damaged"):
        load_documents(FIXTURES / "damaged.pdf", "source-1")


def test_limits_reject_without_a_success_file(tmp_path: Path) -> None:
    huge = tmp_path / "huge.pdf"
    huge.write_bytes(b"%PDF" + b"x" * (10 * 1024 * 1024))
    with pytest.raises(DocumentReadError, match="too_large"):
        load_documents(huge, "source-1")
    many = tmp_path / "many.pdf"
    writer = PdfWriter()
    for _ in range(101):
        writer.add_blank_page(width=72, height=72)
    with many.open("wb") as handle:
        writer.write(handle)
    with pytest.raises(DocumentReadError, match="too_many_pages"):
        load_documents(many, "source-1")
    secret = tmp_path / "secret.pdf"
    locked = PdfWriter()
    locked.add_blank_page(width=72, height=72)
    locked.encrypt("secret")
    with secret.open("wb") as handle:
        locked.write(handle)
    with pytest.raises(DocumentReadError, match="encrypted"):
        load_documents(secret, "source-1")
    assert not (tmp_path / JOB_ID).exists()


def test_timeout_removes_only_the_failed_job(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    def fail(*_args: object, **_kwargs: object) -> None:
        raise subprocess.TimeoutExpired(cmd="dg-content", timeout=0.1)

    monkeypatch.setattr("dg_workflow.adapters.normalize.run_with_stdin", fail)
    monkeypatch.setenv("DG_CONTENT_BIN", "dg-content")
    root = tmp_path / "private"
    failed = root / JOB_ID
    failed.mkdir(parents=True)
    (failed / "partial.txt").write_text("partial", encoding="utf-8")
    other = root / OTHER_JOB
    other.mkdir()
    (other / "keep.txt").write_text("keep", encoding="utf-8")
    source = FIXTURES / "korean.md"
    with pytest.raises(DocumentReadError, match="timeout"):
        ingest_file(source, "source-1", JOB_ID, root, timeout=0.1)
    assert not failed.exists()
    assert (other / "keep.txt").read_text(encoding="utf-8") == "keep"
    assert source.exists()


def test_rust_normalize_is_stable_and_stores_locators(tmp_path: Path) -> None:
    if not os.environ.get("DG_CONTENT_BIN"):
        pytest.skip("DG_CONTENT_BIN is not set")
    root = tmp_path / "private"
    source = FIXTURES / "normal.pdf"
    first = ingest_file(source, "source-1", JOB_ID, root)
    second = ingest_file(source, "source-1", JOB_ID, root)
    assert first["resultHash"] == second["resultHash"]
    assert first["sourceLocator"] == [{"page": 1}, {"page": 2}]
    artifact = (root / JOB_ID / "artifact.json").read_text(encoding="utf-8")
    assert "Field note" in artifact
    assert "Field   note" not in artifact
    script_job = str(uuid.uuid4())
    ingest_file(FIXTURES / "script.md", "source-2", script_job, root)
    script_body = (root / script_job / "artifact.json").read_text(encoding="utf-8")
    assert "<script>alert(1)</script>" in script_body
    assert "html_not_executed" in script_body
    assert "external_include_ignored" in script_body
    receipt = root / JOB_ID / "receipt.json"
    stored = receipt.read_text(encoding="utf-8")
    receipt.write_text(stored.replace(first["resultHash"], "0" * 64), encoding="utf-8")
    before = (root / JOB_ID / "artifact.json").read_bytes()
    with pytest.raises(DocumentReadError, match="artifact_conflict"):
        ingest_file(source, "source-1", JOB_ID, root)
    assert (root / JOB_ID / "artifact.json").read_bytes() == before
