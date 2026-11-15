"""기능: 작은 합성 PDF를 만든다.
목적: 정상 텍스트 PDF와 글자 없는 PDF, 손상 파일을 같은 방식으로 다시 만든다.
연관: `SOURCES.md`, `tests/test_normalize.py`.
"""

from pathlib import Path


def build(objects: list[bytes]) -> bytes:
    header = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n"
    body = bytearray(header)
    offsets = [0]
    for index, obj in enumerate(objects, start=1):
        offsets.append(len(body))
        body.extend(f"{index} 0 obj\n".encode("ascii"))
        body.extend(obj)
        body.extend(b"\nendobj\n")
    startxref = len(body)
    body.extend(f"xref\n0 {len(objects) + 1}\n".encode("ascii"))
    body.extend(b"0000000000 65535 f \n")
    for offset in offsets[1:]:
        body.extend(f"{offset:010d} 00000 n \n".encode("ascii"))
    body.extend(
        f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n".encode("ascii")
    )
    return bytes(body)


def stream(text: str) -> bytes:
    data = text.encode("ascii")
    return f"<< /Length {len(data)} >>\nstream\n".encode("ascii") + data + b"\nendstream"


def page(content_id: int, font_id: int) -> bytes:
    return (
        f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] /Contents {content_id} 0 R "
        f"/Resources << /Font << /F1 {font_id} 0 R >> >> >>"
    ).encode("ascii")


def main() -> None:
    root = Path(__file__).parent
    font = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"
    normal = build(
        [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Count 2 /Kids [3 0 R 5 0 R] >>",
            page(4, 7),
            stream("BT /F1 12 Tf 36 100 Td (Field   note) Tj ET"),
            page(6, 7),
            stream("BT /F1 12 Tf 36 80 Td (Second page) Tj ET"),
            font,
        ]
    )
    image = build(
        [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Contents 4 0 R >>",
            stream("q\nBI /W 1 /H 1 /CS /RGB /BPC 8 ID \x00\x00\x00 EI\nQ"),
        ]
    )
    (root / "normal.pdf").write_bytes(normal)
    (root / "image-only.pdf").write_bytes(image)
    (root / "damaged.pdf").write_bytes(b"%PDF-1.4\nbroken")


if __name__ == "__main__":
    main()
