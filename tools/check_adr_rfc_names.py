#!/usr/bin/env python3
"""Check project ADR/RFC filenames and links to those records."""

from __future__ import annotations

import re
import sys
import unicodedata
from pathlib import Path
from urllib.parse import unquote, urlsplit


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DOCUMENTATION_ROOT = REPOSITORY_ROOT / "docs"
FENCED_CODE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
INLINE_CODE = re.compile(r"(`+)[^`\n]*?\1")
INLINE_LINK = re.compile(r"!?\[[^\]\n]*\]\([ \t]*(?:<([^>\n]+)>|([^\s)]+))")
REFERENCE_LINK = re.compile(
    r"^ {0,3}\[(?!\^)[^\]]+\]:[ \t]*(?:<([^>\n]+)>|([^\s\n]+))",
    re.MULTILINE,
)
HTML_HREF = re.compile(r"\bhref\s*=\s*['\"]([^'\"]+)['\"]", re.IGNORECASE)
ADR_FILE = re.compile(r"^adr-(?P<serial>\d{4})-(?P<slug>[a-z0-9]+(?:-[a-z0-9]+)*)\.md$")
RFC_FILE = re.compile(r"^rfc-(?P<serial>\d{4})-(?P<slug>[a-z0-9]+(?:-[a-z0-9]+)*)\.md$")
ADR_TITLE = re.compile(
    r"^Architectural decision record \(ADR\) (?P<serial>\d{4}):\s*(?P<title>.+)$",
    re.IGNORECASE,
)
RFC_TITLE = re.compile(r"^RFC (?P<serial>\d{4}):\s*(?P<title>.+)$", re.IGNORECASE)
SPECIAL_TRANSLITERATIONS = str.maketrans(
    {
        "Æ": "AE",
        "æ": "ae",
        "Ð": "D",
        "ð": "d",
        "Ł": "L",
        "ł": "l",
        "Ø": "O",
        "ø": "o",
        "Þ": "Th",
        "þ": "th",
        "ß": "ss",
    }
)


def slug_for_title(title: str) -> str:
    """Return the ASCII kebab-case slug used for an ADR or RFC title."""
    transliterated = unicodedata.normalize("NFKD", title.translate(SPECIAL_TRANSLITERATIONS))
    without_marks = "".join(char for char in transliterated if not unicodedata.combining(char))
    ascii_title = without_marks.encode("ascii", "ignore").decode("ascii").lower()
    slug = re.sub(r"[^a-z0-9-]+", "-", ascii_title)
    return re.sub(r"-+", "-", slug).strip("-")


def first_heading(markdown: str) -> str | None:
    """Return the text of the first level-one heading, if the document has one."""
    for line in markdown.splitlines():
        match = re.match(r"^#\s+(.+?)\s*#*\s*$", line)
        if match:
            return match.group(1).strip()
    return None


def extract_markdown_destinations(markdown: str) -> list[str]:
    """Extract inline, reference-definition, and HTML destinations outside code."""
    destinations: list[str] = []
    fenced_char: str | None = None
    fenced_length = 0
    visible_lines: list[str] = []

    for line in markdown.splitlines():
        fence = FENCED_CODE.match(line)
        if fenced_char is None and fence:
            fenced_char = fence.group(1)[0]
            fenced_length = len(fence.group(1))
            continue
        if fenced_char is not None:
            if fence and fence.group(1)[0] == fenced_char and len(fence.group(1)) >= fenced_length:
                fenced_char = None
            continue
        visible_lines.append(INLINE_CODE.sub("", line))

    visible_markdown = "\n".join(visible_lines)
    for pattern in (INLINE_LINK, REFERENCE_LINK):
        for match in pattern.finditer(visible_markdown):
            destination = match.group(1) or match.group(2)
            if destination:
                destinations.append(destination)
    destinations.extend(HTML_HREF.findall(visible_markdown))
    return destinations


def is_record_destination(path: str) -> bool:
    """Return whether a local path refers to a project ADR or RFC filename."""
    decoded = unquote(path).replace("\\", "/")
    path_parts = [part.lower() for part in Path(decoded).parts]
    basename = path_parts[-1] if path_parts else ""
    return (
        basename.startswith(("adr-", "rfc-"))
        or (bool(Path(decoded).suffix) and ("adrs" in path_parts or "rfcs" in path_parts))
    )


def validate_record_files(docs_root: Path) -> list[str]:
    """Check record serials, heading numbers, and title slugs."""
    errors: list[str] = []
    for directory, file_pattern, title_pattern, record_name in (
        (docs_root / "adrs", ADR_FILE, ADR_TITLE, "ADR"),
        (docs_root / "rfcs", RFC_FILE, RFC_TITLE, "RFC"),
    ):
        serials: dict[str, Path] = {}
        for path in sorted(directory.glob("*.md")):
            match = file_pattern.fullmatch(path.name)
            if not match:
                errors.append(f"{path.relative_to(docs_root.parent)}: filename is not canonical")
                continue

            serial = match.group("serial")
            previous = serials.get(serial)
            if previous:
                errors.append(
                    f"{path.relative_to(docs_root.parent)}: duplicate {record_name} serial "
                    f"{serial} also used by {previous.relative_to(docs_root.parent)}"
                )
            else:
                serials[serial] = path

            heading = first_heading(path.read_text(encoding="utf-8"))
            title = title_pattern.fullmatch(heading or "")
            relative_path = path.relative_to(docs_root.parent)
            if not title:
                errors.append(f"{relative_path}: first heading is not a numbered {record_name} title")
                continue
            if title.group("serial") != serial:
                errors.append(
                    f"{relative_path}: filename serial {serial} disagrees with heading serial "
                    f"{title.group('serial')}"
                )
            expected_slug = slug_for_title(title.group("title"))
            if match.group("slug") != expected_slug:
                errors.append(
                    f"{relative_path}: filename slug {match.group('slug')!r} should be "
                    f"{expected_slug!r} for its title"
                )
    return errors


def markdown_files(repository_root: Path) -> list[Path]:
    """List repository Markdown files while excluding generated directories."""
    excluded = {".git", ".venv", "dist", "node_modules", "target", ".uv-cache", ".uv-tools"}
    return sorted(
        path
        for path in repository_root.rglob("*.md")
        if not any(part in excluded for part in path.relative_to(repository_root).parts)
    )


def validate_local_record_links(repository_root: Path, markdown_paths: list[Path]) -> tuple[list[str], int]:
    """Check that local Markdown links targeting ADRs/RFCs resolve to files."""
    errors: list[str] = []
    checked_links = 0
    for source in markdown_paths:
        for destination in extract_markdown_destinations(source.read_text(encoding="utf-8")):
            parsed = urlsplit(destination)
            if parsed.scheme or parsed.netloc or not parsed.path or not is_record_destination(parsed.path):
                continue

            checked_links += 1
            decoded_path = unquote(parsed.path)
            target = (
                repository_root / decoded_path.lstrip("/")
                if decoded_path.startswith("/")
                else source.parent / decoded_path
            ).resolve()
            relative_source = source.relative_to(repository_root)
            try:
                target.relative_to(repository_root.resolve())
            except ValueError:
                errors.append(f"{relative_source}: record link escapes the repository: {destination}")
                continue
            if not target.is_file():
                errors.append(f"{relative_source}: local ADR/RFC link target does not exist: {destination}")
    return errors, checked_links


def main() -> int:
    """Run the repository's ADR/RFC filename and inbound-link checks."""
    errors = validate_record_files(DOCUMENTATION_ROOT)
    paths = markdown_files(REPOSITORY_ROOT)
    link_errors, checked_links = validate_local_record_links(REPOSITORY_ROOT, paths)
    errors.extend(link_errors)

    if errors:
        print("ADR/RFC documentation check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    records = len(list((DOCUMENTATION_ROOT / "adrs").glob("*.md")))
    records += len(list((DOCUMENTATION_ROOT / "rfcs").glob("*.md")))
    print(f"Checked {records} ADR/RFC records and {checked_links} local ADR/RFC links.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
