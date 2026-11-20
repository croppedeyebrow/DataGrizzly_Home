"""기능: 문서 그래프의 노드.
목적: 읽기, 정규화, 검사, 관련 글, 검토 중단, 초안 키 준비를 순서대로 나눈다.
연관: `graph/inspect.py`, `graph/state.py`, `build.py`.
"""

from collections.abc import Callable
from typing import Any

from langgraph.types import interrupt

from dg_workflow.adapters.load import DocumentReadError
from dg_workflow.adapters.retriever import QueryRejected, RelatedLookupError
from dg_workflow.graph.inspect import classify_duplicates, search_query
from dg_workflow.graph.state import (
    SUPPORTED_WORKFLOW_VERSION,
    ReviewState,
    keep_state,
    review_payload,
    workflow_decision,
)


class GraphPorts:
    """기능: 노드가 부르는 바깥 동작.
    목적: 그래프 테스트가 파일과 검색을 대체하고, 초안 저장 횟수를 센다.
    연관: `build_review_graph`.
    """

    def __init__(
        self,
        load: Callable[[str], dict[str, Any]],
        normalize: Callable[[str, str], dict[str, Any]],
        retrieve: Callable[[str], dict[str, Any]],
        existing: list[dict[str, str]] | None = None,
        on_draft: Callable[[str, str], None] | None = None,
    ) -> None:
        self.load = load
        self.normalize = normalize
        self.retrieve = retrieve
        self.existing = existing or []
        self.on_draft = on_draft or _draft_not_stored


def make_nodes(ports: GraphPorts) -> dict[str, Callable[[ReviewState], dict[str, Any]]]:
    """기능: 포트를 닫아 노드 함수를 만든다.
    목적: 체크포인트에는 포트 함수를 넣지 않는다.
    연관: `build_review_graph`.
    """

    def guard(state: ReviewState) -> dict[str, Any]:
        version = int(state.get("workflowVersion", SUPPORTED_WORKFLOW_VERSION))
        if workflow_decision(version) == "isolate":
            return keep_state(
                {
                    "stage": "isolated",
                    "workflowVersion": version,
                    "approvable": False,
                    "issues": [{"code": "workflow_version_isolated"}],
                }
            )
        return keep_state(
            {
                "stateSchemaVersion": 1,
                "workflowVersion": SUPPORTED_WORKFLOW_VERSION,
                "reviewVersion": state.get("reviewVersion", 1),
                "stage": "guard",
            }
        )

    def load(state: ReviewState) -> dict[str, Any]:
        try:
            loaded = ports.load(str(state.get("jobId", "")))
        except DocumentReadError as error:
            return keep_state({"stage": "load", "extractionCode": error.code, "approvable": False})
        update = dict(loaded)
        update["stage"] = "load"
        update["approvable"] = update.get("extractionCode") in (None, "")
        return keep_state(update)

    def normalize(state: ReviewState) -> dict[str, Any]:
        try:
            normalized = ports.normalize(str(state.get("jobId", "")), str(state.get("sourceHash", "")))
        except DocumentReadError as error:
            return keep_state({"stage": "normalize", "extractionCode": error.code, "approvable": False})
        update = dict(normalized)
        update["stage"] = "normalize"
        if update.get("extractionCode"):
            update["approvable"] = False
        return keep_state(update)

    def inspect(state: ReviewState) -> dict[str, Any]:
        metadata = _metadata(state)
        title = str(metadata.get("title", ""))
        issues = classify_duplicates(str(state.get("sourceHash", "")), title, ports.existing)
        if not title:
            issues.append({"code": "missing_title", "existingId": ""})
        return keep_state({"stage": "inspect", "issues": issues})

    def retrieve(state: ReviewState) -> dict[str, Any]:
        metadata = _metadata(state)
        query = search_query(
            _strings(metadata.get("keywords")),
            _strings(metadata.get("tags")),
            str(metadata.get("title", "")),
        )
        if not query:
            return keep_state({"stage": "retrieve", "relatedEntryIds": []})
        try:
            found = ports.retrieve(query)
        except QueryRejected:
            return keep_state({"stage": "retrieve", "warnings": ["query_rejected"], "relatedEntryIds": []})
        except RelatedLookupError:
            return keep_state({"stage": "retrieve", "warnings": ["relatedLookupFailed"], "relatedEntryIds": []})
        return keep_state(
            {
                "stage": "retrieve",
                "relatedEntryIds": list(found.get("relatedEntryIds", [])),
                "warnings": list(found.get("warnings", [])),
            }
        )

    def review(state: ReviewState) -> dict[str, Any]:
        payload = review_payload(state)
        interrupt(payload)
        return keep_state({"stage": "review", "reviewVersion": payload["reviewVersion"]})

    def save_draft(state: ReviewState) -> dict[str, Any]:
        job_id = str(state.get("jobId", ""))
        key = f"{job_id}:draft"
        if state.get("approvable"):
            ports.on_draft(job_id, key)
        return keep_state({"stage": "save_draft", "idempotencyKey": key, "draftId": None})

    return {
        "guard": guard,
        "load": load,
        "normalize": normalize,
        "inspect": inspect,
        "retrieve": retrieve,
        "review": review,
        "save_draft": save_draft,
    }


def after_guard(state: dict[str, Any]) -> str:
    """기능: 버전이 다르면 읽기를 시작하지 않는다.
    목적: 구버전 작업을 새 thread로 옮기지 않는다.
    연관: `workflow_decision`.
    """
    if state.get("stage") == "isolated":
        return "end"
    return "load"


def after_load(state: dict[str, Any]) -> str:
    """기능: 읽기 실패면 정규화를 건너뛴다.
    목적: 추출 오류와 중복 이슈를 검토에서 보게 한다.
    연관: `extractionCode`.
    """
    if state.get("extractionCode"):
        return "review"
    return "normalize"


def after_normalize(state: dict[str, Any]) -> str:
    """기능: 정규화 실패면 관련 글 조회를 건너뛴다.
    목적: 원문 추출 실패를 검색 장애와 섞지 않는다.
    연관: `after_load`.
    """
    if state.get("extractionCode"):
        return "review"
    return "inspect"


def after_review(state: dict[str, Any]) -> str:
    """기능: 승인 가능한 검토만 초안 노드로 보낸다.
    목적: 추출 실패 상태에서는 초안 키 준비를 하지 않는다.
    연관: `save_draft`.
    """
    if state.get("approvable"):
        return "save_draft"
    return "end"


def _draft_not_stored(job_id: str, key: str) -> None:
    _ = (job_id, key)


def _metadata(state: ReviewState) -> dict[str, Any]:
    raw = state.get("metadata")
    if isinstance(raw, dict):
        return raw
    return {}


def _strings(value: Any) -> list[str]:
    if not isinstance(value, list):
        return []
    return [item for item in value if isinstance(item, str)]
