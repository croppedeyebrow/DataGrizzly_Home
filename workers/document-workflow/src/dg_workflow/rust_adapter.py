"""기능: OS별로 Rust CLI를 실행하는 방법을 나눈다.
목적: Windows 경로를 WSL 경로로 추정하지 않고, 계약 버전만 확인한다.
연관: `tools/content-cli`의 `contract`와 `validate -`, `shell.py`.
"""

import json
import os
import subprocess
from pathlib import Path

from dg_workflow.shell import CONTRACT_SCHEMA_VERSION, ShellError


def linux_binary_argv(distro: str, linux_binary: str, args: list[str]) -> list[str]:
    """기능: WSL 안의 Linux 바이너리 인자만 만든다.
    목적: 드라이브 문자나 백슬래시 경로를 Linux 경로로 바꾸지 않는다.
    연관: `DG_CONTENT_LINUX_PATH`.
    """
    if not linux_binary.startswith("/") or "\\" in linux_binary or ":" in linux_binary:
        raise ValueError("WSL binary must be a linux path")
    return ["wsl.exe", "-d", distro, "--", linux_binary, *args]


def native_argv(binary: str, args: list[str]) -> list[str]:
    """기능: 현재 OS에서 직접 실행할 인자만 만든다.
    목적: 바이너리 위치를 환경값으로 받고 경로를 추측하지 않는다.
    연관: `DG_CONTENT_BIN`.
    """
    return [binary, *args]


def contract_argv() -> list[str]:
    """기능: 계약 확인에 쓸 명령 인자를 고른다.
    목적: WSL 경로가 있을 때만 WSL 실행을 사용한다.
    연관: `linux_binary_argv`, `native_argv`.
    """
    distro = os.environ.get("DG_CONTENT_WSL_DISTRO", "")
    linux_binary = os.environ.get("DG_CONTENT_LINUX_PATH", "")
    if distro or linux_binary:
        if not distro or not linux_binary:
            raise ShellError(2, "DG_CONTENT_WSL_DISTRO and DG_CONTENT_LINUX_PATH are required")
        return linux_binary_argv(distro, linux_binary, ["contract"])
    binary = os.environ.get("DG_CONTENT_BIN", "")
    if not binary:
        raise ShellError(2, "DG_CONTENT_BIN is required")
    return native_argv(binary, ["contract"])


def contract_from_binary() -> dict[str, object]:
    """기능: Rust `contract` 출력의 schemaVersion을 읽는다.
    목적: 워커가 기대하는 계약 1과 바이너리 출력이 같은지 확인한다.
    연관: `tools/content-core`의 schemaVersion 1.
    """
    completed = run_with_stdin(contract_argv(), b"", timeout=5)
    if completed.returncode != 0:
        raise ShellError(1, "rust contract command failed")
    return read_contract(completed.stdout.decode("utf-8"))


def read_contract(stdout: str) -> dict[str, object]:
    """기능: 계약 JSON에서 schemaVersion만 확인한다.
    목적: 다른 버전을 같은 계약으로 받아들이지 않는다.
    연관: `CONTRACT_SCHEMA_VERSION`.
    """
    try:
        payload = json.loads(stdout)
    except json.JSONDecodeError as error:
        raise ShellError(1, "rust contract output is not json") from error
    if not isinstance(payload, dict) or payload.get("schemaVersion") != CONTRACT_SCHEMA_VERSION:
        raise ShellError(1, "schemaVersion must be 1")
    return {"schemaVersion": CONTRACT_SCHEMA_VERSION}


def run_with_stdin(
    argv: list[str],
    payload: bytes,
    timeout: float,
    scratch: Path | None = None,
) -> subprocess.CompletedProcess[bytes]:
    """기능: JSON을 표준입력으로 넘기고, 실패하면 임시 파일만 지운다.
    목적: 원본 문서와 WSL 경로를 정리 대상에 넣지 않는다.
    연관: `dg-content validate -`.
    """
    temp = None
    if scratch is not None:
        scratch.mkdir(parents=True, exist_ok=True)
        temp = scratch / "stdin.json"
        temp.write_bytes(payload)
    try:
        return subprocess.run(argv, input=payload, capture_output=True, timeout=timeout, check=False)
    except (subprocess.TimeoutExpired, OSError):
        if temp is not None:
            temp.unlink(missing_ok=True)
        raise
    finally:
        if temp is not None and temp.exists():
            temp.unlink(missing_ok=True)
