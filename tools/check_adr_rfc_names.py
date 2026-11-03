#!/usr/bin/env python3
"""Check canonical ADR/RFC filenames and links to those records.

This checker helps keep record titles, serials, filenames, and local references
consistent. Run it directly with ``python3 tools/check_adr_rfc_names.py`` or
through ``make check-adr-rfc-names`` to run its tests and repository checks.
"""

from __future__ import annotations

import re
import sys
import unicodedata
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import SplitResult, unquote, urlsplit


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DOCUMENTATION_ROOT = REPOSITORY_ROOT / "docs"
FENCED_CODE = re.compile(r"^ {0,3}(`{3,}|~{3,})(.*)$")
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


@dataclass(frozen=True)
class RecordKind:
    """Describe the filename and heading rules for one record family."""

    docs_root: Path
    directory: Path
    file_pattern: re.Pattern[str]
    title_pattern: re.Pattern[str]
    name: str


def slug_for_title(title: str) -> str:
    """Return the ASCII kebab-case slug used for an ADR or RFC title.

    Example: `slug_for_title("Adopt latest guide")` returns
    `"adopt-latest-guide"`.
    """
    transliterated = unicodedata.normalize("NFKD", title.translate(SPECIAL_TRANSLITERATIONS))
    without_marks = "".join(char for char in transliterated if not unicodedata.combining(char))
    ascii_title = without_marks.encode("ascii", "ignore").decode("ascii").lower()
    slug = re.sub(r"[^a-z0-9-]+", "-", ascii_title)
    return re.sub(r"-+", "-", slug).strip("-")


def first_heading(markdown: str) -> str | None:
    """Return the text of the first level-one heading, if present.

    Example: `first_heading("# Decision\nBody")` returns `"Decision"`.
    """
    for line in markdown.splitlines():
        match = re.match(r"^#\s+(.+?)\s*#*\s*$", line)
        if match:
            return match.group(1).strip()
    return None


def _closes_fence(
    fence: re.Match[str] | None,
    fenced_char: str,
    fenced_length: int,
) -> bool:
    """Return whether a fence marker closes the current block.

    Example: a four-backtick marker closes an open three-backtick fence.
    """
    if fence is None:
        return False
    marker = fence.group(1)
    if marker[0] != fenced_char:
        return False
    return len(marker) >= fenced_length and not fence.group(2).strip()


def _visible_markdown(markdown: str) -> str:
    """Remove fenced and inline code while keeping visible Markdown links.

    Example: a link inside a fenced block is omitted from the result.
    """
    fenced_char: str | None = None
    fenced_length = 0
    visible_lines: list[str] = []

    for line in markdown.splitlines():
        fence = FENCED_CODE.match(line)
        if fenced_char is not None:
            if _closes_fence(fence, fenced_char, fenced_length):
                fenced_char = None
            continue
        if fence is None:
            visible_lines.append(INLINE_CODE.sub("", line))
            continue
        fenced_char = fence.group(1)[0]
        fenced_length = len(fence.group(1))

    return "\n".join(visible_lines)


def _destinations_for_pattern(pattern: re.Pattern[str], markdown: str) -> list[str]:
    """Collect nonempty link destinations matched by one Markdown pattern.

    Example: an inline-link pattern extracts `record.md` from `[x](record.md)`.
    """
    destinations: list[str] = []
    for match in pattern.finditer(markdown):
        destination = match.group(1) or match.group(2)
        if destination:
            destinations.append(destination)
    return destinations


def extract_markdown_destinations(markdown: str) -> list[str]:
    """Extract inline, reference-definition, and HTML destinations outside code.

    Example: `[x](record.md)` contributes `record.md` to the result.
    """
    visible_markdown = _visible_markdown(markdown)
    destinations: list[str] = []
    for pattern in (INLINE_LINK, REFERENCE_LINK):
        destinations.extend(_destinations_for_pattern(pattern, visible_markdown))
    destinations.extend(HTML_HREF.findall(visible_markdown))
    return destinations


def is_record_destination(path: str) -> bool:
    """Return whether a local path refers to a project ADR or RFC filename.

    Example: `is_record_destination("docs/adrs/adr-0001-example.md")` returns
    `True`.
    """
    decoded = unquote(path).replace("\\", "/")
    path_parts = [part.lower() for part in Path(decoded).parts]
    basename = path_parts[-1] if path_parts else ""
    return (
        basename.startswith(("adr-", "rfc-"))
        or (bool(Path(decoded).suffix) and ("adrs" in path_parts or "rfcs" in path_parts))
    )


def _record_file_match(
    path: Path,
    file_pattern: re.Pattern[str],
) -> re.Match[str] | None:
    """Return a canonical filename match, if one exists.

    Example: a canonical `adr-0001-example.md` filename returns a match.
    """
    return file_pattern.fullmatch(path.name)


def _register_serial(
    serials: dict[str, Path],
    serial: str,
    path: Path,
    record_kind: RecordKind,
) -> str | None:
    """Record a serial once and return an error for a duplicate.

    Example: registering serial `0001` twice returns a duplicate-serial error.
    """
    previous = serials.get(serial)
    if previous:
        return (
            f"{path.relative_to(record_kind.docs_root.parent)}: duplicate "
            f"{record_kind.name} serial {serial} also used by "
            f"{previous.relative_to(record_kind.docs_root.parent)}"
        )
    serials[serial] = path
    return None


def _record_heading_errors(
    path: Path,
    serial: str,
    filename_slug: str,
    record_kind: RecordKind,
) -> list[str]:
    """Check one record heading against its filename serial and title slug.

    Example: a heading for `0001: Example` requires the slug `example`.
    """
    relative_path = path.relative_to(record_kind.docs_root.parent)
    heading = first_heading(path.read_text(encoding="utf-8"))
    title = record_kind.title_pattern.fullmatch(heading or "")
    if not title:
        return [
            f"{relative_path}: first heading is not a numbered "
            f"{record_kind.name} title"
        ]

    errors: list[str] = []
    if title.group("serial") != serial:
        errors.append(
            f"{relative_path}: filename serial {serial} disagrees with heading serial "
            f"{title.group('serial')}"
        )
    expected_slug = slug_for_title(title.group("title"))
    if filename_slug != expected_slug:
        errors.append(
            f"{relative_path}: filename slug {filename_slug!r} should be "
            f"{expected_slug!r} for its title"
        )
    return errors


def validate_record_files(docs_root: Path) -> list[str]:
    """Check record serials, heading numbers, and title slugs.

    Example: duplicate serials or a slug that disagrees with a title are errors.
    """
    errors: list[str] = []
    record_kinds = (
        RecordKind(docs_root, docs_root / "adrs", ADR_FILE, ADR_TITLE, "ADR"),
        RecordKind(docs_root, docs_root / "rfcs", RFC_FILE, RFC_TITLE, "RFC"),
    )
    for record_kind in record_kinds:
        serials: dict[str, Path] = {}
        for path in sorted(record_kind.directory.glob("*.md")):
            match = _record_file_match(path, record_kind.file_pattern)
            if match is None:
                errors.append(
                    f"{path.relative_to(docs_root.parent)}: filename is not canonical"
                )
                continue

            serial = match.group("serial")
            serial_error = _register_serial(serials, serial, path, record_kind)
            if serial_error:
                errors.append(serial_error)
            errors.extend(
                _record_heading_errors(
                    path,
                    serial,
                    match.group("slug"),
                    record_kind,
                )
            )
    return errors


def markdown_files(repository_root: Path) -> list[Path]:
    """List repository Markdown files while excluding generated directories.

    Example: `markdown_files(Path("/repo"))` omits `/repo/target/output.md`.
    """
    excluded = {".git", ".venv", "dist", "node_modules", "target", ".uv-cache", ".uv-tools"}
    markdown_paths: list[Path] = []
    directories = [repository_root]
    while directories:
        directory = directories.pop()
        for path in directory.iterdir():
            if path.is_symlink():
                if path.suffix == ".md":
                    markdown_paths.append(path)
                continue
            if path.is_dir():
                if path.name not in excluded:
                    directories.append(path)
            elif path.is_file() and path.suffix == ".md":
                markdown_paths.append(path)
    return sorted(markdown_paths)


def _is_external_destination(parsed_url: SplitResult) -> bool:
    """Return whether a parsed URL points outside the local Markdown tree.

    Example: `https://example.test/adr.md` is external.
    """
    return bool(parsed_url.scheme) or bool(parsed_url.netloc)


def _resolve_local_target(repository_root: Path, source: Path, link_path: str) -> Path:
    """Resolve a Markdown path from its source or repository root.

    Example: `/docs/adrs/adr-0001-example.md` resolves from the repository root.
    """
    decoded_path = unquote(link_path)
    if decoded_path.startswith("/"):
        return (repository_root / decoded_path.lstrip("/")).resolve()
    return (source.parent / decoded_path).resolve()


def _local_record_target(
    repository_root: Path,
    source: Path,
    destination: str,
) -> Path | None:
    """Resolve a local ADR/RFC destination, or return `None` when unrelated.

    Example: an HTTP link returns `None`; a relative ADR link returns its path.
    """
    parsed_url = urlsplit(destination)
    if _is_external_destination(parsed_url):
        return None
    if not parsed_url.path:
        return None
    if not is_record_destination(parsed_url.path):
        return None
    return _resolve_local_target(repository_root, source, parsed_url.path)


def _local_record_link_error(
    repository_root: Path,
    source: Path,
    target: Path,
    destination: str,
) -> str | None:
    """Describe a local record link that escapes the repository or is missing.

    Example: a link to an absent ADR returns a path-specific error.
    """
    relative_source = source.relative_to(repository_root)
    try:
        target.relative_to(repository_root.resolve())
    except ValueError:
        return f"{relative_source}: record link escapes the repository: {destination}"
    if target.is_file():
        return None
    return f"{relative_source}: local ADR/RFC link target does not exist: {destination}"


def validate_local_record_links(
    repository_root: Path,
    markdown_paths: list[Path],
) -> tuple[list[str], int]:
    """Check that local Markdown links targeting ADRs/RFCs resolve to files.

    Example: a missing local ADR destination appears in the returned errors.
    """
    errors: list[str] = []
    checked_links = 0
    for source in markdown_paths:
        markdown = source.read_text(encoding="utf-8")
        for destination in extract_markdown_destinations(markdown):
            target = _local_record_target(repository_root, source, destination)
            if target is None:
                continue
            checked_links += 1
            error = _local_record_link_error(repository_root, source, target, destination)
            if error:
                errors.append(error)
    return errors, checked_links


def main() -> int:
    """Run the repository's ADR/RFC filename and inbound-link checks.

    Example: `main()` returns `0` when every record and local link is valid.
    """
    try:
        errors = validate_record_files(DOCUMENTATION_ROOT)
        paths = markdown_files(REPOSITORY_ROOT)
        link_errors, checked_links = validate_local_record_links(REPOSITORY_ROOT, paths)
        errors.extend(link_errors)

        records = len(list((DOCUMENTATION_ROOT / "adrs").glob("*.md")))
        records += len(list((DOCUMENTATION_ROOT / "rfcs").glob("*.md")))
    except (OSError, UnicodeError) as error:
        print(
            f"ADR/RFC documentation check could not read repository files: {error}",
            file=sys.stderr,
        )
        return 1

    if errors:
        print("ADR/RFC documentation check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(f"Checked {records} ADR/RFC records and {checked_links} local ADR/RFC links.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
