"""기능: PDF 한 파일에서 텍스트를 읽는다.
목적: langchain community 없이 pypdf로 문서 읽기를 시작할 자리를 둔다.
연관: `cli.py`. 접수 API와 그래프는 S21 이후다.
"""

from pathlib import Path

from pypdf import PdfReader


class PdfTextLoader:
    def __init__(self, path: Path) -> None:
        self.path = path

    def load(self) -> str:
        if not self.path.is_file():
            raise FileNotFoundError(self.path)
        reader = PdfReader(str(self.path))
        return "\n".join(page.extract_text() or "" for page in reader.pages)
