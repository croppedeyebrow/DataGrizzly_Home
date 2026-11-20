"""기능: 검토 그래프의 중단, 재개, 버전 분리를 확인한다.
목적: 같은 작업 번호로 검토를 다시 열고, 상태에 원문과 토큰이 남지 않게 한다.
연관: `dg_workflow.graph.build`, `langgraph.types.Command`.
"""

import json
import uuid
from pathlib import Path
from typing import Any

import pytest
from langgraph.checkpoint.memory import MemorySaver
from langgraph.errors import GraphInterrupt
from langgraph.types import Command

from dg_workflow.adapters.load import DocumentReadError
from dg_workflow.adapters.retriever import RelatedLookupError
from dg_workflow.graph.build import build_review_graph
from dg_workflow.graph.nodes import GraphPorts
from dg_workflow.graph.state import dump_state, review_payload, workflow_decision

SECRET = "비공개 원문 전체와 token-value"
SOURCE_HASH = "ab" * 32
ARTIFACT_ID = "cd" * 32


def _ports(drafts: list[str], *, fail: str = "", crash: Path | None = None) -> GraphPorts:
    def load(job_id: str) -> dict[str, Any]:
        _ = job_id
        if fail == "load":
            raise DocumentReadError("damaged")
        return {
            "sourceHash": SOURCE_HASH,
            "metadata": {"title": "현장 메모", "keywords": ["데이터"]},
            "sectionRefs": [{"heading": "측정", "lineStart": 1, "lineEnd": 2}],
            "page_content": SECRET,
        }

    def normalize(job_id: str, source_hash: str) -> dict[str, Any]:
        _ = (job_id, source_hash)
        if crash is not None:
            crash.write_text(ARTIFACT_ID, encoding="utf-8")
            raise RuntimeError("killed")
        return {"artifactId": ARTIFACT_ID, "text": SECRET}

    def retrieve(query: str) -> dict[str, Any]:
        if fail == "search":
            raise RelatedLookupError
        assert SECRET not in query
        return {"relatedEntryIds": ["bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb1"]}

    return GraphPorts(load, normalize, retrieve, on_draft=lambda job_id, key: drafts.append(f"{job_id}:{key}"))


def _config(job_id: str) -> dict[str, Any]:
    return {"configurable": {"thread_id": job_id}}


def _pause(graph, job_id: str, version: int = 1):
    config = _config(job_id)
    try:
        graph.invoke({"jobId": job_id, "workflowVersion": version, "reviewVersion": 1}, config)
    except GraphInterrupt:
        pass
    return graph.get_state(config)


def _interrupt_value(snapshot) -> dict[str, Any]:
    interrupts = getattr(snapshot, "interrupts", None) or []
    if interrupts:
        return interrupts[0].value
    for task in snapshot.tasks:
        if task.interrupts:
            return task.interrupts[0].value
    raise AssertionError("review interrupt is missing")


def test_workflow_version_one_resumes_and_others_stay_isolated() -> None:
    assert workflow_decision(1) == "resume"
    assert workflow_decision(0) == "isolate"
    assert workflow_decision(2) == "isolate"


def test_review_pauses_without_draft_and_hides_source() -> None:
    drafts: list[str] = []
    job_id = str(uuid.uuid4())
    graph = build_review_graph(MemorySaver(), _ports(drafts, fail=""))
    snapshot = _pause(graph, job_id)
    payload = _interrupt_value(snapshot)
    assert payload["jobId"] == job_id
    assert payload["reviewVersion"] == 1
    assert payload["sectionRefs"] == [{"heading": "측정", "lineStart": 1, "lineEnd": 2}]
    assert payload["issues"] == []
    assert payload["approvable"] is True
    assert snapshot.config["configurable"]["thread_id"] == job_id
    assert drafts == []
    encoded = dump_state(snapshot.values) + json.dumps(payload, ensure_ascii=False)
    assert SECRET not in encoded
    assert "page_content" not in snapshot.values
    assert "text" not in snapshot.values


def test_resume_in_a_new_graph_uses_the_same_thread() -> None:
    drafts: list[str] = []
    job_id = str(uuid.uuid4())
    saver = MemorySaver()
    first = build_review_graph(saver, _ports(drafts))
    paused = _pause(first, job_id)
    assert _interrupt_value(paused)["relatedEntryIds"] == ["bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb1"]
    second = build_review_graph(saver, _ports(drafts))
    resumed = second.get_state(_config(job_id))
    assert _interrupt_value(resumed)["reviewVersion"] == 1
    assert second.get_state(_config(str(uuid.uuid4()))).values == {}
    second.invoke(Command(resume={"action": "approve"}), _config(job_id))
    assert drafts == [f"{job_id}:{job_id}:draft"]
    assert second.get_state(_config(job_id)).values["artifactId"] == ARTIFACT_ID
    assert second.get_state(_config(job_id)).values["draftId"] is None


def test_duplicate_is_an_issue_and_does_not_overwrite_the_artifact(tmp_path: Path) -> None:
    drafts: list[str] = []
    receipt = tmp_path / "receipt.txt"
    receipt.write_text(ARTIFACT_ID, encoding="utf-8")
    existing = [{"id": "job-existing", "sha256": SOURCE_HASH, "title": "다른 제목"}]
    ports = _ports(drafts)
    ports.existing = existing
    job_id = str(uuid.uuid4())
    graph = build_review_graph(MemorySaver(), ports)
    payload = _interrupt_value(_pause(graph, job_id))
    assert payload["issues"] == [{"code": "confirmed_duplicate", "existingId": "job-existing"}]
    assert drafts == []
    assert receipt.read_text(encoding="utf-8") == ARTIFACT_ID


def test_extraction_failure_is_not_approvable_and_skips_draft() -> None:
    drafts: list[str] = []
    job_id = str(uuid.uuid4())
    graph = build_review_graph(MemorySaver(), _ports(drafts, fail="load"))
    payload = _interrupt_value(_pause(graph, job_id))
    assert payload["approvable"] is False
    graph.invoke(Command(resume={"action": "approve"}), _config(job_id))
    assert drafts == []
    assert graph.get_state(_config(job_id)).values["extractionCode"] == "damaged"


def test_search_failure_keeps_review_available() -> None:
    drafts: list[str] = []
    job_id = str(uuid.uuid4())
    graph = build_review_graph(MemorySaver(), _ports(drafts, fail="search"))
    payload = _interrupt_value(_pause(graph, job_id))
    assert payload["warnings"] == ["relatedLookupFailed"]
    assert payload["approvable"] is True
    assert SECRET not in json.dumps(payload, ensure_ascii=False)


def test_killed_normalize_keeps_the_artifact_hash(tmp_path: Path) -> None:
    marker = tmp_path / "artifact.txt"
    drafts: list[str] = []
    job_id = str(uuid.uuid4())
    saver = MemorySaver()
    graph = build_review_graph(saver, _ports(drafts, crash=marker))
    with pytest.raises(RuntimeError, match="killed"):
        graph.invoke({"jobId": job_id, "workflowVersion": 1}, _config(job_id))
    restored = build_review_graph(saver, _ports(drafts)).get_state(_config(job_id))
    assert restored.values["sourceHash"] == SOURCE_HASH
    assert marker.read_text(encoding="utf-8") == ARTIFACT_ID
    assert SECRET not in dump_state(restored.values)


def test_old_workflow_version_does_not_start_another_thread() -> None:
    drafts: list[str] = []
    called: list[str] = []

    def load(job_id: str) -> dict[str, Any]:
        called.append(job_id)
        return {"sourceHash": SOURCE_HASH}

    ports = GraphPorts(load, lambda _job, _source: {}, lambda _query: {})
    job_id = str(uuid.uuid4())
    graph = build_review_graph(MemorySaver(), ports)
    graph.invoke({"jobId": job_id, "workflowVersion": 0}, _config(job_id))
    state = graph.get_state(_config(job_id))
    assert state.values["stage"] == "isolated"
    assert state.values["issues"][0]["code"] == "workflow_version_isolated"
    assert state.config["configurable"]["thread_id"] == job_id
    assert called == []
    assert drafts == []
    encoded = review_payload(state.values)
    assert "token-value" not in json.dumps(encoded)


def test_postgres_checkpoint_resumes_after_the_connection_closes() -> None:
    pytest.importorskip("psycopg")
    from dg_workflow.checkpoint import runtime_dsn, runtime_saver

    try:
        runtime_dsn()
    except RuntimeError:
        pytest.skip("checkpoint database is not configured")
    drafts: list[str] = []
    job_id = str(uuid.uuid4())
    import psycopg

    try:
        with runtime_saver() as saver:
            graph = build_review_graph(saver, _ports(drafts))
            paused = _pause(graph, job_id)
            assert _interrupt_value(paused)["jobId"] == job_id
            assert drafts == []
        with runtime_saver() as saver:
            graph = build_review_graph(saver, _ports(drafts))
            restored = graph.get_state(_config(job_id))
            assert restored.values["sourceHash"] == SOURCE_HASH
            assert _interrupt_value(restored)["reviewVersion"] == 1
            assert restored.config["configurable"]["thread_id"] == job_id
            graph.invoke(Command(resume={"action": "approve"}), _config(job_id))
            assert drafts == [f"{job_id}:{job_id}:draft"]
            assert SECRET not in dump_state(graph.get_state(_config(job_id)).values)
    except psycopg.OperationalError:
        pytest.skip("checkpoint database is unavailable")
