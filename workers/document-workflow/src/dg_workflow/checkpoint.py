"""기능: workflow 스키마에 체크포인트를 만들고, 런타임 계정으로 값을 다시 읽는다.
목적: 관리 계정과 실행 계정을 나누고, setup을 그래프 실행마다 하지 않는다.
연관: `langgraph.checkpoint.postgres.PostgresSaver`, `settings.py`, `cli.py`, `infra/ops/backup.mjs`.
"""

from collections.abc import Iterator
from contextlib import contextmanager
from typing import TypedDict
from urllib.parse import quote, urlparse

import psycopg
from langgraph.checkpoint.postgres import PostgresSaver
from langgraph.graph import END, START, StateGraph
from psycopg import sql
from psycopg.rows import dict_row

from dg_workflow.settings import load_settings

CHECKPOINT_TABLES = ("checkpoint_migrations", "checkpoints", "checkpoint_blobs", "checkpoint_writes")


class Probe(TypedDict):
    value: str


def with_workflow_schema(dsn: str) -> str:
    if "search_path" in dsn:
        return dsn
    separator = "&" if "?" in dsn else "?"
    return f"{dsn}{separator}options=-csearch_path%3Dworkflow"


def assert_local_database(dsn: str) -> None:
    parsed = urlparse(dsn)
    database = parsed.path.strip("/")
    if parsed.hostname != "127.0.0.1" or parsed.port != 15432 or database != "datagrizzly":
        raise RuntimeError("checkpoint target must be 127.0.0.1:15432/datagrizzly")


def admin_dsn() -> str:
    settings = load_settings()
    if settings["WORKFLOW_SETUP_DSN"]:
        return settings["WORKFLOW_SETUP_DSN"]
    password = os_password("POSTGRES_PASSWORD")
    return f"postgresql://datagrizzly:{quote(password, safe='')}@127.0.0.1:15432/datagrizzly"


def runtime_dsn() -> str:
    settings = load_settings()
    if settings["WORKFLOW_CHECKPOINT_DSN"]:
        return settings["WORKFLOW_CHECKPOINT_DSN"]
    password = os_password("WORKFLOW_RUNTIME_PASSWORD")
    return f"postgresql://workflow_runtime:{quote(password, safe='')}@127.0.0.1:15432/datagrizzly"


def setup_checkpoint() -> dict[str, object]:
    """기능: 체크포인트 테이블과 런타임 권한을 한 번 맞춘다.
    목적: 그래프를 열 때마다 migration을 반복하지 않는다.
    연관: `PostgresSaver.setup`, CMS 스키마 `public`.
    """
    password = os_password("WORKFLOW_RUNTIME_PASSWORD")
    dsn = admin_dsn()
    assert_local_database(dsn)
    with connect(dsn) as conn:
        ensure_runtime_role(conn, password)
        conn.execute("CREATE SCHEMA IF NOT EXISTS workflow")
        conn.execute("REVOKE ALL ON SCHEMA workflow FROM PUBLIC")
        conn.execute("SET search_path TO workflow")
        PostgresSaver(conn).setup()
        conn.execute("GRANT CONNECT ON DATABASE datagrizzly TO workflow_runtime")
        conn.execute("GRANT USAGE ON SCHEMA workflow TO workflow_runtime")
        conn.execute(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA workflow TO workflow_runtime"
        )
        conn.execute("REVOKE ALL ON ALL TABLES IN SCHEMA public FROM workflow_runtime")
        schemas = {
            row["table_schema"]
            for row in conn.execute(
                "SELECT table_schema FROM information_schema.tables WHERE table_name = 'checkpoints'"
            ).fetchall()
        }
    if schemas != {"workflow"}:
        raise RuntimeError("checkpoints must stay in the workflow schema")
    return {"ok": True, "schema": "workflow", "tables": list(CHECKPOINT_TABLES)}


def save_probe(thread_id: str, value: str) -> None:
    """기능: 짧은 상태 하나를 저장한다.
    목적: 이 함수는 setup을 호출하지 않는다.
    연관: `read_probe`.
    """
    with runtime_saver() as saver:
        build_graph(saver).invoke({"value": value}, {"configurable": {"thread_id": thread_id}})


def read_probe(thread_id: str) -> str:
    """기능: 같은 thread_id의 저장값을 읽는다.
    목적: 프로세스가 끝난 뒤에도 체크포인트가 남는지 본다.
    연관: `save_probe`.
    """
    with runtime_saver() as saver:
        state = build_graph(saver).get_state({"configurable": {"thread_id": thread_id}})
    return str(state.values.get("value", ""))


def cms_write_denied() -> str:
    """기능: 런타임 계정으로 CMS 테이블 변경을 시도한다.
    목적: public 스키마 쓰기가 권한 오류인지 확인한다.
    연관: `public.entry`.
    """
    dsn = runtime_dsn()
    assert_local_database(dsn)
    try:
        with connect(with_workflow_schema(dsn)) as conn:
            conn.execute("UPDATE public.entry SET title = title WHERE false")
    except psycopg.errors.InsufficientPrivilege:
        return "insufficient_privilege"
    return "cms_write_allowed"


def build_graph(saver: PostgresSaver):
    builder = StateGraph(Probe)
    builder.add_node("keep", lambda state: {"value": state["value"]})
    builder.add_edge(START, "keep")
    builder.add_edge("keep", END)
    return builder.compile(checkpointer=saver)


@contextmanager
def runtime_saver() -> Iterator[PostgresSaver]:
    dsn = with_workflow_schema(runtime_dsn())
    assert_local_database(dsn)
    with PostgresSaver.from_conn_string(dsn) as saver:
        yield saver


def connect(dsn: str):
    return psycopg.Connection.connect(dsn, autocommit=True, prepare_threshold=0, row_factory=dict_row)


def ensure_runtime_role(conn, password: str) -> None:
    exists = conn.execute("SELECT 1 FROM pg_roles WHERE rolname = 'workflow_runtime'").fetchone()
    verb = "ALTER ROLE" if exists else "CREATE ROLE"
    conn.execute(
        sql.SQL("{} workflow_runtime LOGIN PASSWORD {} NOSUPERUSER NOCREATEDB NOCREATEROLE").format(
            sql.SQL(verb),
            sql.Literal(password),
        )
    )


def os_password(name: str) -> str:
    import os

    value = os.environ.get(name, "")
    if not value:
        raise RuntimeError(f"{name} is required")
    return value
