"""기능: 문서 그래프의 상태.
목적: 버전과 원문 참조만 남기고, 원문 전체와 토큰은 상태에 넣지 않는다.
연관: `WF03`, `nodes.py`, `build.py`.
"""

import json
from typing import Any, TypedDict

STATE_SCHEMA_VERSION = 1
SUPPORTED_WORKFLOW_VERSION = 1

STATE_KEYS = (
    "stateSchemaVersion",
    "workflowVersion",
    "jobId",
    "sourceHash",
    "artifactId",
    "sectionRefs",
    "metadata",
    "issues",
    "relatedEntryIds",
    "reviewVersion",
    "draftId",
    "warnings",
    "extractionCode",
    "approvable",
    "stage",
    "idempotencyKey",
)


class ReviewState(TypedDict, total=False):
    """기능: 체크포인트에 저장하는 검토 상태.
    목적: 작업 번호, 원문 hash, 절 참조, 이슈만 재개에 쓴다.
    연관: `review_payload`.
    """

    stateSchemaVersion: int
    workflowVersion: int
    jobId: str
    sourceHash: str
    artifactId: str
    sectionRefs: list[dict[str, Any]]
    metadata: dict[str, Any]
    issues: list[dict[str, str]]
    relatedEntryIds: list[str]
    reviewVersion: int
    draftId: str | None
    warnings: list[str]
    extractionCode: str | None
    approvable: bool
    stage: str
    idempotencyKey: str


def workflow_decision(version: int) -> str:
    """기능: 저장된 작업 버전을 재개할지 고른다.
    목적: 지원하는 버전 1만 이어서 실행하고, 다른 버전은 그 체크포인트에 둔다.
    연관: W17, `SUPPORTED_WORKFLOW_VERSION`.
    """
    if version == SUPPORTED_WORKFLOW_VERSION:
        return "resume"
    return "isolate"


def keep_state(update: dict[str, Any]) -> dict[str, Any]:
    """기능: 상태 갱신에서 허용된 칸만 남긴다.
    목적: 원문, 토큰, 비밀번호 칸이 체크포인트로 들어가지 않게 한다.
    연관: `STATE_KEYS`.
    """
    return {key: value for key, value in update.items() if key in STATE_KEYS}


def review_payload(state: ReviewState | dict[str, Any]) -> dict[str, Any]:
    """기능: 검토 중단에 보여줄 값.
    목적: 버전, 메타데이터, 발췌 참조, 관련 글, 이슈만 전달한다.
    연관: `review` 노드의 `interrupt`.
    """
    return {
        "reviewVersion": state.get("reviewVersion", 1),
        "metadata": state.get("metadata", {}),
        "sectionRefs": state.get("sectionRefs", []),
        "relatedEntryIds": state.get("relatedEntryIds", []),
        "issues": state.get("issues", []),
        "warnings": state.get("warnings", []),
        "approvable": bool(state.get("approvable")),
        "jobId": state.get("jobId", ""),
        "workflowVersion": state.get("workflowVersion", SUPPORTED_WORKFLOW_VERSION),
    }


def dump_state(state: dict[str, Any]) -> str:
    """기능: 상태를 JSON 문자열로 바꾼다.
    목적: 직렬화 결과에 허용되지 않은 칸이 없는지 검사한다.
    연관: `keep_state`.
    """
    return json.dumps(keep_state(state), ensure_ascii=False, sort_keys=True)
