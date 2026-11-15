"""기능: 워커 환경값을 프로세스 환경에서만 읽는다.
목적: `.env` 파일을 자동으로 불러오지 않고, 비밀값은 실행 환경에서만 받는다.
연관: `.env.example`, `checkpoint.py`, `cli.py`.
"""

import os

SETTING_NAMES = (
    "CMS_INTERNAL_BASE_URL",
    "WORKFLOW_SETUP_DSN",
    "WORKFLOW_CHECKPOINT_DSN",
    "WORKFLOW_TOKEN",
    "WORKFLOW_VERSION",
    "WORKFLOW_MAX_JOBS",
    "WORKFLOW_RUNTIME_PASSWORD",
)


def load_settings() -> dict[str, str]:
    return {name: os.environ.get(name, "") for name in SETTING_NAMES}
