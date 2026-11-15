"""기능: 로그에서 접속 비밀번호와 토큰을 가린다.
목적: 체크포인트 실패 기록이 DSN 비밀번호나 WORKFLOW_TOKEN을 남기지 않게 한다.
연관: `cli.py`, `checkpoint.py`.
"""

import os
import re


def redact(text: str) -> str:
    hidden = re.sub(
        r"(postgres(?:ql)?://[^:\s/@]+:)[^@\s]+@",
        r"\1[redacted]@",
        text,
        flags=re.IGNORECASE,
    )
    hidden = re.sub(r"(?i)(password=)[^\s&]+", r"\1[redacted]", hidden)
    token = os.environ.get("WORKFLOW_TOKEN", "")
    if token:
        hidden = hidden.replace(token, "[redacted]")
    return hidden
