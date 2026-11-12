"""Validate canonical ADR/RFC filenames, headings, and local links.

The rules keep record titles, serials, filenames, and local references
consistent. They operate on domain values only: the ``RecordReader`` port in
``adr_rfc_ports.py`` supplies record text and link containment facts, so this
module imports no filesystem path type. ``tools/check_adr_rfc_names.py`` is the
command-line entry point; ``make check-adr-rfc-names`` runs it after this
module's tests.
"""

from __future__ import annotations

import re
import unicodedata
from dataclasses import dataclass
from urllib.parse import SplitResult, unquote, urlsplit

from adr_rfc_ports import LinkFacts, RecordFile, RecordReader


FENCED_CODE = re.compile(r"^ {0,3}(`{3,}|~{3,})(.*)$")
INLINE_CODE = re.compile(r"(`+)[^`\n]*?\1")
INLINE_LINK = re.compile(
    r"!?\[[^\]]*?\]\([ \t]*(?:\n[ \t]*)?(?:<([^>\n]+)>|([^\s)]+))"
)
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
    """Describe the filename and heading rules for one record family.

    Example: the ADR family is `RecordKind("adrs", ADR_FILE, ADR_TITLE, "ADR")`.
    """

    family: str
    file_pattern: re.Pattern[str]
    title_pattern: re.Pattern[str]
    name: str


RECORD_KINDS = (
    RecordKind("adrs", ADR_FILE, ADR_TITLE, "ADR"),
    RecordKind("rfcs", RFC_FILE, RFC_TITLE, "RFC"),
)


@dataclass(frozen=True)
class RecordLink:
    """Describe a local Markdown link that targets a candidate record.

    Attributes
    ----------
    source : str
        Repository-relative path of the document that holds the link.
    destination : str
        Destination exactly as written in the document, used in diagnostics.
    facts : LinkFacts | None
        What the adapter established about the target, or `None` when the
        destination cannot be interpreted as a path.

    Example: `[x](adrs/adr-0001-example.md)` yields a link whose `facts` say
    whether the destination is inside the repository and exists.
    """

    source: str
    destination: str
    facts: LinkFacts | None


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

    The path is a link destination, so it is split as text rather than handed to
    a filesystem path type.

    Example: `is_record_destination("docs/adrs/adr-0001-example.md")` returns
    `True`.
    """
    decoded = unquote(path).replace("\\", "/")
    path_parts = [part.lower() for part in decoded.split("/") if part not in ("", ".")]
    basename = path_parts[-1] if path_parts else ""
    return basename.startswith(("adr-", "rfc-")) or (
        bool(_file_suffix(basename))
        and ("adrs" in path_parts or "rfcs" in path_parts)
    )


def _file_suffix(basename: str) -> str:
    """Return a filename's suffix, treating dotfiles as suffixless.

    Example: `_file_suffix("adr-0001-example.md")` returns `"md"`, while
    `_file_suffix(".gitkeep")` returns `""`.
    """
    stem, dot, suffix = basename.rpartition(".")
    return suffix if dot and stem else ""


def _register_serial(
    serials: dict[str, str],
    serial: str,
    record: RecordFile,
    record_kind: RecordKind,
) -> str | None:
    """Record a serial once and return an error for a duplicate.

    Example: registering serial `0001` twice returns a duplicate-serial error.
    """
    previous = serials.get(serial)
    if previous:
        return (
            f"{record.relative_path}: duplicate {record_kind.name} serial "
            f"{serial} also used by {previous}"
        )
    serials[serial] = record.relative_path
    return None


def _record_heading_errors(
    record: RecordFile,
    filename_match: re.Match[str],
    record_kind: RecordKind,
) -> list[str]:
    """Check one record heading against its filename serial and title slug.

    Example: a heading for `0001: Example` requires the slug `example`.
    """
    heading = first_heading(record.text)
    title = record_kind.title_pattern.fullmatch(heading or "")
    if not title:
        return [
            (
                f"{record.relative_path}: first heading is not a numbered "
                f"{record_kind.name} title"
            )
        ]

    serial = filename_match.group("serial")
    filename_slug = filename_match.group("slug")
    errors: list[str] = []
    if title.group("serial") != serial:
        errors.append(
            f"{record.relative_path}: filename serial {serial} disagrees with "
            f"heading serial {title.group('serial')}"
        )
    expected_slug = slug_for_title(title.group("title"))
    if filename_slug != expected_slug:
        errors.append(
            f"{record.relative_path}: filename slug {filename_slug!r} should be "
            f"{expected_slug!r} for its title"
        )
    return errors


def validate_record_files(reader: RecordReader) -> list[str]:
    """Check record serials, heading numbers, and title slugs.

    Example: duplicate serials or a slug that disagrees with a title are errors.
    """
    errors: list[str] = []
    for record_kind in RECORD_KINDS:
        serials: dict[str, str] = {}
        for record in reader.record_files(record_kind.family):
            match = record_kind.file_pattern.fullmatch(record.name)
            if match is None:
                errors.append(f"{record.relative_path}: filename is not canonical")
                continue

            serial = match.group("serial")
            serial_error = _register_serial(serials, serial, record, record_kind)
            if serial_error:
                errors.append(serial_error)
            errors.extend(_record_heading_errors(record, match, record_kind))
    return errors


def _is_external_destination(parsed_url: SplitResult) -> bool:
    """Return whether a parsed URL points outside the local Markdown tree.

    Example: `https://example.test/adr.md` is external.
    """
    return bool(parsed_url.scheme) or bool(parsed_url.netloc)


def _local_record_link(
    reader: RecordReader,
    source: str,
    destination: str,
) -> RecordLink | None:
    """Resolve a local ADR/RFC destination, or return `None` when unrelated.

    Example: an HTTP link returns `None`; a relative ADR link returns its
    containment facts.
    """
    try:
        parsed_url = urlsplit(destination)
    except ValueError:
        # A malformed URL such as `http://[abc` is not a usable link. The
        # Markdown linter reports it, so the record checker ignores it.
        return None
    if _is_external_destination(parsed_url):
        return None
    if not parsed_url.path:
        return None
    if not is_record_destination(parsed_url.path):
        return None
    facts = reader.link_facts(source, parsed_url.path)
    return RecordLink(source, destination, facts)


def _local_record_link_error(link: RecordLink) -> str | None:
    """Describe a local record link that escapes the repository or is missing.

    Example: a link to an absent ADR returns a path-specific error.
    """
    if link.facts is None:
        return f"{link.source}: malformed local record link: {link.destination}"
    if not link.facts.inside_root:
        return f"{link.source}: record link escapes the repository: {link.destination}"
    if not link.facts.exists:
        return (
            f"{link.source}: local ADR/RFC link target does not exist: "
            f"{link.destination}"
        )
    return None


def validate_local_record_links(reader: RecordReader) -> tuple[list[str], int]:
    """Check that local Markdown links targeting ADRs/RFCs resolve to files.

    Example: a missing local ADR destination appears in the returned errors.
    """
    errors: list[str] = []
    checked_links = 0
    for document in reader.markdown_documents():
        for destination in extract_markdown_destinations(document.text):
            link = _local_record_link(reader, document.relative_path, destination)
            if link is None:
                continue
            checked_links += 1
            error = _local_record_link_error(link)
            if error:
                errors.append(error)
    return errors, checked_links
