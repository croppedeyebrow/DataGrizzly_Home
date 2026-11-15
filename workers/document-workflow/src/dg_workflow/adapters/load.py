"""기능: PDF와 Markdown을 LangChain Document로 읽는다.
목적: 페이지 번호와 제목·줄 범위를 남기고, OCR이나 모델 호출은 하지 않는다.
연관: `pypdf`, `langchain_core.documents.Document`, `limits.py`.
"""

from pathlib import Path

from langchain_core.documents import Document
from pypdf import PdfReader

from dg_workflow.adapters.limits import MARKDOWN_BYTES, PDF_BYTES, PDF_PAGES, TEXT_CHARS


class DocumentReadError(Exception):
    """기능: 읽기 실패 코드만 담는다.
    목적: 원문과 경로 비밀을 오류 문장에 넣지 않는다.
    연관: `pipeline.py`의 실패 정리.
    """

    def __init__(self, code: str) -> None:
        super().__init__(code)
        self.code = code


def load_documents(path: Path, source_id: str) -> list[Document]:
    """기능: 확장자에 따라 PDF 또는 Markdown을 읽는다.
    목적: 허용되지 않은 형식은 내용을 해석하지 않고 거절한다.
    연관: `load_pdf`, `load_markdown`.
    """
    suffix = path.suffix.lower()
    if suffix == ".pdf":
        return load_pdf(path, source_id)
    if suffix == ".md":
        return load_markdown(path, source_id)
    raise DocumentReadError("unsupported_type")


def load_pdf(path: Path, source_id: str) -> list[Document]:
    """기능: PDF 각 페이지의 텍스트와 페이지 번호를 읽는다.
    목적: 손상, 암호화, 이미지 전용, 한도 초과는 빈 성공 결과로 바꾸지 않는다.
    연관: `pypdf.PdfReader`.
    """
    size = _size(path)
    if size > PDF_BYTES:
        raise DocumentReadError("too_large")
    try:
        reader = PdfReader(str(path))
    except Exception as error:
        raise _damaged(error) from error
    if reader.is_encrypted:
        raise DocumentReadError("encrypted")
    page_count = len(reader.pages)
    if page_count > PDF_PAGES:
        raise DocumentReadError("too_many_pages")
    documents: list[Document] = []
    saw_text = False
    for index, page in enumerate(reader.pages, start=1):
        try:
            text = page.extract_text() or ""
        except Exception as error:
            raise _damaged(error) from error
        if text.strip():
            saw_text = True
        documents.append(Document(page_content=text, metadata={"sourceId": source_id, "kind": "pdf", "page": index}))
    if not saw_text:
        raise DocumentReadError("image_only")
    _reject_long_text("".join(document.page_content for document in documents))
    return documents


def load_markdown(path: Path, source_id: str) -> list[Document]:
    """기능: Markdown을 제목 단위로 나누고 줄 번호를 남긴다.
    목적: HTML을 실행하거나 외부 파일을 가져오지 않는다.
    연관: `TEXT_CHARS`, `MARKDOWN_BYTES`.
    """
    size = _size(path)
    if size > MARKDOWN_BYTES:
        raise DocumentReadError("too_large")
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError as error:
        raise DocumentReadError("encoding") from error
    _reject_long_text(text)
    lines = text.splitlines()
    sections: list[tuple[str, int, int, list[str]]] = []
    heading = ""
    start = 1
    buffer: list[str] = []
    for number, line in enumerate(lines, start=1):
        if line.startswith("#"):
            if buffer or heading:
                sections.append((heading, start, number - 1, buffer))
            heading = line.lstrip("#").strip()
            start = number
            buffer = []
        else:
            buffer.append(line)
    sections.append((heading, start, max(start, len(lines)), buffer))
    documents: list[Document] = []
    for section_heading, line_start, line_end, body in sections:
        content = "\n".join(body).strip("\n")
        if not content and not section_heading:
            continue
        documents.append(
            Document(
                page_content=content,
                metadata={
                    "sourceId": source_id,
                    "kind": "markdown",
                    "heading": section_heading,
                    "lineStart": line_start,
                    "lineEnd": line_end,
                },
            )
        )
    if not documents:
        raise DocumentReadError("empty_text")
    return documents


def _damaged(error: Exception) -> Exception:
    if isinstance(error, DocumentReadError):
        return error
    if isinstance(error, (OSError, ValueError)) or type(error).__module__.startswith("pypdf"):
        return DocumentReadError("damaged")
    return error


def _size(path: Path) -> int:
    if not path.is_file():
        raise DocumentReadError("missing_file")
    return path.stat().st_size


def _reject_long_text(text: str) -> None:
    if len(text) > TEXT_CHARS:
        raise DocumentReadError("too_many_chars")
