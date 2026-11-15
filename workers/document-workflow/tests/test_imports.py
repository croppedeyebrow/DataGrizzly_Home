"""기능: 워커 환경의 import 확인.
목적: 빈 테스트만 통과한 상태를 환경 완료로 보지 않는다.
연관: `dg_workflow.loader`, `langgraph`, `langchain_core`, `pypdf`.
"""

from pathlib import Path

import langchain_core
import langgraph
import pypdf

from dg_workflow.loader import PdfTextLoader


def test_runtime_imports_and_missing_pdf_is_rejected() -> None:
    assert langgraph.__name__ == "langgraph"
    assert langchain_core.__name__ == "langchain_core"
    assert pypdf.__name__ == "pypdf"
    missing = Path("private") / "missing.pdf"
    try:
        PdfTextLoader(missing).load()
    except FileNotFoundError as error:
        assert "missing.pdf" in str(error)
        return
    raise AssertionError("missing pdf was accepted")
