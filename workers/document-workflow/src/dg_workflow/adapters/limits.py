"""기능: 문서 읽기 한도.
목적: 크기, 페이지, 글자 수, 실행 시간을 넘긴 입력을 성공으로 저장하지 않는다.
연관: `docs/11_document_workflow/WF05_SECURITY_LIMITS.md`, `load.py`.
"""

PDF_BYTES = 10 * 1024 * 1024
PDF_PAGES = 100
MARKDOWN_BYTES = 2 * 1024 * 1024
TEXT_CHARS = 1_000_000
EXTRACT_TIMEOUT_SECONDS = 60
PROCESSING_VERSION = 1
# Rust `QUERY_MAX_CHARS`와 같은 한도다. 질문을 더 길게 바꾸지 않는다.
QUERY_MAX_CHARS = 80
SEARCH_TIMEOUT_SECONDS = 2
