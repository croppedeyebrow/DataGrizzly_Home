"""기능: 문서 검토 그래프를 조립한다.
목적: load에서 save_draft까지 잇고, 검토 노드에서만 실행을 멈춘다.
연관: `nodes.py`, `PostgresSaver`, `WF03`.
"""

from typing import Any, cast

from langgraph.checkpoint.base import BaseCheckpointSaver
from langgraph.graph import END, START, StateGraph

from dg_workflow.graph.nodes import GraphPorts, after_guard, after_load, after_normalize, after_review, make_nodes
from dg_workflow.graph.state import ReviewState


def build_review_graph(checkpointer: BaseCheckpointSaver, ports: GraphPorts):
    """기능: 체크포인터를 넣은 검토 그래프를 만든다.
    목적: thread_id는 호출자가 작업 번호로 넘긴다. 이 함수는 초안 테이블을 쓰지 않는다.
    연관: `runtime_saver`, `interrupt`.
    """
    nodes = make_nodes(ports)
    builder = StateGraph(ReviewState)
    for name in ("guard", "load", "normalize", "inspect", "retrieve", "review", "save_draft"):
        builder.add_node(name, cast(Any, nodes[name]))
    builder.add_edge(START, "guard")
    builder.add_conditional_edges("guard", after_guard, {"load": "load", "end": END})
    builder.add_conditional_edges("load", after_load, {"normalize": "normalize", "review": "review"})
    builder.add_conditional_edges("normalize", after_normalize, {"inspect": "inspect", "review": "review"})
    builder.add_edge("inspect", "retrieve")
    builder.add_edge("retrieve", "review")
    builder.add_conditional_edges("review", after_review, {"save_draft": "save_draft", "end": END})
    builder.add_edge("save_draft", END)
    return builder.compile(checkpointer=checkpointer)
