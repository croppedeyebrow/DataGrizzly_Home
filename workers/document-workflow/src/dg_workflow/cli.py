"""기능: `dg-workflow` 명령의 진입점.
목적: 패키지가 설치됐는지 확인하고, 문서 작업을 실행한 것처럼 보이지 않게 한다.
연관: `pyproject.toml`의 `project.scripts`, `__init__.py`.
"""

from dg_workflow import __version__


def main() -> None:
    print(f"dg-workflow {__version__} environment")
