"""기능: 실행 골격의 종료 코드, 환경 확인, Rust 어댑터를 검사한다.
목적: 작업이 없을 때의 정상 종료와 비밀값 미출력을 고정한다.
연관: `dg_workflow.shell`, `dg_workflow.rust_adapter`, `fixtures/synthetic-note.json`.
"""

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

from dg_workflow.rust_adapter import linux_binary_argv, read_contract, run_with_stdin
from dg_workflow.shell import require_shell_env, run_shell

FIXTURE = Path(__file__).parents[1] / "fixtures" / "synthetic-note.json"


def shell_env(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("CMS_INTERNAL_BASE_URL", "http://127.0.0.1:8080")
    monkeypatch.setenv("WORKFLOW_TOKEN", "token-value")
    monkeypatch.setenv("WORKFLOW_VERSION", "1")
    monkeypatch.setenv("WORKFLOW_MAX_JOBS", "1")


def test_help_lists_run_flags() -> None:
    completed = subprocess.run(
        [sys.executable, "-m", "dg_workflow.cli", "--help"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
    )
    assert completed.returncode == 0
    assert "--once" in completed.stdout
    assert "--drain" in completed.stdout
    assert "--job" in completed.stdout


def test_missing_env_exits_before_work(monkeypatch: pytest.MonkeyPatch) -> None:
    for name in ("CMS_INTERNAL_BASE_URL", "WORKFLOW_TOKEN", "WORKFLOW_VERSION", "WORKFLOW_MAX_JOBS"):
        monkeypatch.delenv(name, raising=False)
    monkeypatch.setenv("WORKFLOW_TOKEN", "token-value")
    code = run_shell(["--once"])
    assert code == 2


def test_public_url_is_rejected_without_printing_secrets(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    shell_env(monkeypatch)
    monkeypatch.setenv("CMS_INTERNAL_BASE_URL", "http://user:super-secret@example.com")
    code = run_shell(["--once"])
    captured = capsys.readouterr()
    assert code == 2
    assert "super-secret" not in captured.err
    assert "token-value" not in captured.err
    assert "example.com" not in captured.err
    monkeypatch.setenv("CMS_INTERNAL_BASE_URL", "https://example.com")
    assert run_shell(["--once"]) == 2
    captured = capsys.readouterr()
    assert "example.com" not in captured.err
    assert "127.0.0.1" in captured.err


def test_no_job_exits_zero(monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]) -> None:
    shell_env(monkeypatch)
    assert run_shell(["--once"]) == 0
    assert json.loads(capsys.readouterr().out)["processed"] == 0
    assert run_shell(["--drain"]) == 0
    assert json.loads(capsys.readouterr().out)["processed"] == 0
    assert run_shell(["--job", "missing-job"]) == 0
    assert json.loads(capsys.readouterr().out)["processed"] == 0


def test_synthetic_document_prints_schema(monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]) -> None:
    shell_env(monkeypatch)
    assert run_shell(["--once", "--document", str(FIXTURE)]) == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload == {"schemaVersion": 1, "command": "once", "processed": 1, "title": "합성 메모"}


def test_internal_api_must_be_loopback(monkeypatch: pytest.MonkeyPatch) -> None:
    shell_env(monkeypatch)
    require_shell_env()
    monkeypatch.setenv("CMS_INTERNAL_BASE_URL", "https://127.0.0.1:8080")
    with pytest.raises(Exception, match="127.0.0.1"):
        require_shell_env()


def test_wsl_binary_must_be_a_linux_path() -> None:
    argv = linux_binary_argv("Ubuntu", "/usr/local/bin/dg-content", ["validate", "-"])
    assert argv == ["wsl.exe", "-d", "Ubuntu", "--", "/usr/local/bin/dg-content", "validate", "-"]
    with pytest.raises(ValueError, match="linux path"):
        linux_binary_argv("Ubuntu", r"C:\tools\dg-content.exe", ["contract"])


def test_contract_json_requires_schema_version_one() -> None:
    assert read_contract('{"schemaVersion":1}\n') == {"schemaVersion": 1}
    with pytest.raises(Exception, match="schemaVersion"):
        read_contract('{"schemaVersion":2}')


def test_timeout_removes_only_the_temp_file(tmp_path: Path) -> None:
    source = tmp_path / "source.json"
    source.write_text("{}", encoding="utf-8")
    scratch = tmp_path / "scratch"
    with pytest.raises(subprocess.TimeoutExpired):
        run_with_stdin([sys.executable, "-c", "import time; time.sleep(30)"], b"{}", 1, scratch)
    assert not (scratch / "stdin.json").exists()
    assert source.read_text(encoding="utf-8") == "{}"
