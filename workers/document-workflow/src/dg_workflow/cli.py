"""기능: `dg-workflow` 명령의 진입점.
목적: 환경 확인, 체크포인트 setup, 저장, 읽기, CMS 권한 확인을 서로 다른 명령으로 나눈다.
연관: `pyproject.toml`의 `project.scripts`, `checkpoint.py`, `redact.py`.
"""

import json
import sys

from dg_workflow import __version__
from dg_workflow.checkpoint import cms_write_denied, read_probe, save_probe, setup_checkpoint
from dg_workflow.redact import redact


def main() -> None:
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    try:
        if command == "":
            print(f"dg-workflow {__version__} environment")
            return
        if command == "setup-checkpoint":
            print(json.dumps(setup_checkpoint()))
            return
        if command == "checkpoint-write":
            save_probe(thread_id(sys.argv), sys.argv[3])
            print(json.dumps({"ok": True, "threadId": sys.argv[2]}))
            return
        if command == "checkpoint-read":
            print(json.dumps({"ok": True, "threadId": thread_id(sys.argv), "value": read_probe(thread_id(sys.argv))}))
            return
        if command == "checkpoint-probe-cms":
            code = cms_write_denied()
            print(json.dumps({"ok": code == "insufficient_privilege", "code": code}))
            if code != "insufficient_privilege":
                raise SystemExit(1)
            return
        raise RuntimeError("unknown command")
    except SystemExit:
        raise
    except Exception as error:
        print(redact(str(error)), file=sys.stderr)
        raise SystemExit(1) from None


def thread_id(argv: list[str]) -> str:
    if len(argv) < 3 or not argv[2]:
        raise RuntimeError("thread id is required")
    return argv[2]
