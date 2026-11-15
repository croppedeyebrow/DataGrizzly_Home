"""기능: 워커를 한 번, 비울 때까지, 또는 지정 작업으로 시작하고 끝낸다.
목적: 대기 작업이 없어도 정상 종료하고, 환경이 없으면 일을 시작하기 전에 멈춘다.
연관: `cli.py`, `settings.py`, `rust_adapter.py`, `fixtures/synthetic-note.json`.
"""

import argparse
import json
import sys
from pathlib import Path
from urllib.parse import urlparse

from dg_workflow.redact import redact
from dg_workflow.settings import load_settings

CONTRACT_SCHEMA_VERSION = 1
REQUIRED_SETTINGS = ("CMS_INTERNAL_BASE_URL", "WORKFLOW_TOKEN", "WORKFLOW_VERSION", "WORKFLOW_MAX_JOBS")


class ShellError(Exception):
    """기능: 실행 골격의 종료 코드와 공개 가능한 문장을 함께 담는다.
    목적: 비밀값이 섞인 예외 문자열을 그대로 출력하지 않는다.
    연관: `cli.py`의 종료 코드.
    """

    def __init__(self, code: int, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


def run_shell(argv: list[str]) -> int:
    """기능: `--once`, `--drain`, `--job`, `--rust-contract` 중 하나를 실행한다.
    목적: help와 종료 코드를 명령 진입점에서 고정한다.
    연관: `python -m dg_workflow.cli`.
    """
    parser = argparse.ArgumentParser(prog="dg-workflow")
    parser.add_argument("--once", action="store_true", help="대기 작업이 있으면 하나만 처리한다")
    parser.add_argument("--drain", action="store_true", help="대기 작업이 없을 때까지 처리한다")
    parser.add_argument("--job", default="", help="지정한 작업만 처리한다")
    parser.add_argument("--document", default="", help="합성 문서 JSON 파일")
    parser.add_argument("--rust-contract", action="store_true", help="Rust CLI 계약 버전을 확인한다")
    try:
        args = parser.parse_args(argv)
        command = selected_command(args.once, args.drain, args.job, args.rust_contract)
        require_shell_env()
        if args.rust_contract:
            from dg_workflow.rust_adapter import contract_from_binary

            print(json.dumps(contract_from_binary(), ensure_ascii=False))
            return 0
        if args.document:
            print(json.dumps(read_document(Path(args.document), command), ensure_ascii=False))
            return 0
        print(
            json.dumps(
                {"schemaVersion": CONTRACT_SCHEMA_VERSION, "command": command, "processed": 0},
                ensure_ascii=False,
            )
        )
        return 0
    except ShellError as error:
        print(redact(error.message), file=sys.stderr)
        return error.code


def selected_command(once: bool, drain: bool, job: str, rust_contract: bool) -> str:
    """기능: 실행 방식을 하나로 고른다.
    목적: 한 번에 두 실행 방식이 겹치지 않게 한다.
    연관: `run_shell`.
    """
    modes = (
        ("once", once),
        ("drain", drain),
        ("job", bool(job)),
        ("rust-contract", rust_contract),
    )
    chosen = [name for name, enabled in modes if enabled]
    if len(chosen) != 1:
        raise ShellError(2, "one of --once, --drain, --job, --rust-contract is required")
    return "job" if job else chosen[0]


def require_shell_env() -> dict[str, str]:
    """기능: 실행 전에 필요한 환경값만 확인한다.
    목적: 로컬 API 주소와 공개 주소를 구분하고, 값이 없으면 일을 시작하지 않는다.
    연관: `settings.load_settings`, `.env.example`.
    """
    settings = load_settings()
    for name in REQUIRED_SETTINGS:
        if not settings[name]:
            raise ShellError(2, f"{name} is required")
    if settings["WORKFLOW_VERSION"] != "1":
        raise ShellError(2, "WORKFLOW_VERSION must be 1")
    if settings["WORKFLOW_MAX_JOBS"] != "1":
        raise ShellError(2, "WORKFLOW_MAX_JOBS must be 1")
    parsed = urlparse(settings["CMS_INTERNAL_BASE_URL"])
    if parsed.username or parsed.password:
        raise ShellError(2, "CMS_INTERNAL_BASE_URL must not include credentials")
    if parsed.scheme != "http" or parsed.hostname != "127.0.0.1":
        raise ShellError(2, "CMS_INTERNAL_BASE_URL must be http://127.0.0.1")
    return settings


def read_document(path: Path, command: str) -> dict[str, object]:
    """기능: 로컬 합성 문서를 읽어 결과 JSON을 만든다.
    목적: 네트워크에서 문서를 가져오지 않고 schemaVersion 1만 통과시킨다.
    연관: `fixtures/synthetic-note.json`, `tools/content-core`의 schemaVersion 1.
    """
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ShellError(1, "document is not readable json") from error
    if not isinstance(payload, dict) or payload.get("schemaVersion") != CONTRACT_SCHEMA_VERSION:
        raise ShellError(1, "document schemaVersion must be 1")
    title = payload.get("title")
    if not isinstance(title, str) or not title:
        raise ShellError(1, "document title is required")
    return {"schemaVersion": CONTRACT_SCHEMA_VERSION, "command": command, "processed": 1, "title": title}
