"""기능: 체크포인트 명령의 비밀값과 setup 경계를 확인한다.
목적: 데이터베이스 없이도 마스킹, 환경 파일 미로딩, setup 분리를 고정한다.
연관: `dg_workflow.redact`, `dg_workflow.settings`, `dg_workflow.checkpoint`.
"""

import inspect
from pathlib import Path

from dg_workflow.checkpoint import assert_local_database, save_probe, setup_checkpoint, with_workflow_schema
from dg_workflow.redact import redact
from dg_workflow.settings import load_settings


def test_redact_hides_dsn_password_and_token(monkeypatch) -> None:
    monkeypatch.setenv("WORKFLOW_TOKEN", "token-value")
    hidden = redact("postgresql://datagrizzly:secret-pass@127.0.0.1:15432/datagrizzly token-value password=secret-pass")
    assert "secret-pass" not in hidden
    assert "token-value" not in hidden
    assert "[redacted]" in hidden


def test_settings_do_not_read_dotenv(monkeypatch, tmp_path: Path) -> None:
    monkeypatch.delenv("WORKFLOW_TOKEN", raising=False)
    (tmp_path / ".env").write_text("WORKFLOW_TOKEN=file-token\n", encoding="utf-8")
    monkeypatch.chdir(tmp_path)
    assert load_settings()["WORKFLOW_TOKEN"] == ""


def test_checkpoint_target_stays_on_the_local_database() -> None:
    assert_local_database("postgresql://workflow_runtime@127.0.0.1:15432/datagrizzly")
    try:
        assert_local_database("postgresql://workflow_runtime@127.0.0.1:5432/datagrizzly")
    except RuntimeError as error:
        assert "15432" in str(error)
        return
    raise AssertionError("other port was accepted")


def test_schema_option_is_added_once() -> None:
    once = with_workflow_schema("postgresql://workflow_runtime@127.0.0.1:15432/datagrizzly")
    assert "search_path%3Dworkflow" in once
    assert with_workflow_schema(once) == once


def test_save_probe_does_not_call_setup() -> None:
    assert ".setup(" not in inspect.getsource(save_probe)
    assert ".setup(" in inspect.getsource(setup_checkpoint)
