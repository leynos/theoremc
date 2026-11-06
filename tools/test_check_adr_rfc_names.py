"""Tests for the ADR/RFC documentation contract checker."""

from __future__ import annotations

from pathlib import Path

import pytest

from adr_rfc_names import (
    extract_markdown_destinations,
    slug_for_title,
    validate_local_record_links,
    validate_record_files,
)
from adr_rfc_repository import PathRecordReader, markdown_files


def test_slug_transliterates_accents_and_keeps_kebab_case() -> None:
    assert slug_for_title("Café and Æther") == "cafe-and-aether"


def test_extracts_local_links_but_ignores_code() -> None:
    markdown = """
[record](adrs/adr-0001-example.md#decision)
[reference]: <rfcs/rfc-0001-example.md>
`[inline code](adrs/ignored.md)`

```markdown
[fenced code](adrs/also-ignored.md)
```
"""

    assert extract_markdown_destinations(markdown) == [
        "adrs/adr-0001-example.md#decision",
        "rfcs/rfc-0001-example.md",
    ]


def test_extracts_wrapped_link_text_and_destination() -> None:
    markdown = (
        "- [ADR-0002: Library-first internationalization and localization with\n"
        "  Fluent](\n"
        "adrs/adr-0002-library-first-internationalization-and-localization-with-fluent.md\n"
        ")\n"
    )

    assert extract_markdown_destinations(markdown) == [
        "adrs/adr-0002-library-first-internationalization-and-localization-with-fluent.md"
    ]


def test_fence_with_info_string_does_not_close_code_block() -> None:
    markdown = """
```markdown
```python
[hidden](docs/adrs/adr-0001-missing.md)
```
[visible](docs/adrs/adr-0002-visible.md)
```
"""

    assert extract_markdown_destinations(markdown) == [
        "docs/adrs/adr-0002-visible.md"
    ]


def test_footnote_continuations_are_not_reference_links() -> None:
    markdown = """
[^2]: `ADR-0001-3`
      (docs/adrs/adr-0001-example.md decision 3)
"""

    assert extract_markdown_destinations(markdown) == []


def test_reports_duplicate_record_serials(tmp_path: Path) -> None:
    adrs_root = tmp_path / "docs" / "adrs"
    adrs_root.mkdir(parents=True)
    (adrs_root / "adr-0001-first-decision.md").write_text(
        "# Architectural decision record (ADR) 0001: First decision\n",
        encoding="utf-8",
    )
    (adrs_root / "adr-0001-second-decision.md").write_text(
        "# Architectural decision record (ADR) 0001: Second decision\n",
        encoding="utf-8",
    )

    errors = validate_record_files(PathRecordReader(), tmp_path / "docs")

    assert len(errors) == 1
    assert "duplicate ADR serial 0001" in errors[0]


def test_checks_noncanonical_names_and_rfc_contract(tmp_path: Path) -> None:
    adrs_root = tmp_path / "docs" / "adrs"
    rfcs_root = tmp_path / "docs" / "rfcs"
    adrs_root.mkdir(parents=True)
    rfcs_root.mkdir()
    (adrs_root / "adr-1-noncanonical.md").write_text("", encoding="utf-8")
    (rfcs_root / "rfc-12-noncanonical.md").write_text("", encoding="utf-8")
    (rfcs_root / "rfc-0001-first-proposal.md").write_text(
        "# RFC 0001: First proposal\n", encoding="utf-8"
    )
    (rfcs_root / "rfc-0001-second-proposal.md").write_text(
        "# RFC 0001: Second proposal\n", encoding="utf-8"
    )
    (rfcs_root / "rfc-0003-wrong-title.md").write_text(
        "# RFC 0004: Canonical title\n", encoding="utf-8"
    )

    errors = validate_record_files(PathRecordReader(), tmp_path / "docs")

    assert sum("filename is not canonical" in error for error in errors) == 2
    assert any("duplicate RFC serial 0001" in error for error in errors)
    assert any("disagrees with heading serial 0004" in error for error in errors)
    assert any("should be 'canonical-title'" in error for error in errors)


@pytest.mark.parametrize(
    ("family", "filename", "heading"),
    (
        ("adrs", "adr-0001-example.md", ""),
        ("adrs", "adr-0001-example.md", "# Not an ADR title\n"),
        ("rfcs", "rfc-0001-example.md", ""),
        ("rfcs", "rfc-0001-example.md", "# Not an RFC title\n"),
    ),
)
def test_reports_missing_or_malformed_headings(
    tmp_path: Path, family: str, filename: str, heading: str
) -> None:
    records_root = tmp_path / "docs" / family
    records_root.mkdir(parents=True)
    (records_root / filename).write_text(heading, encoding="utf-8")

    errors = validate_record_files(PathRecordReader(), tmp_path / "docs")

    assert len(errors) == 1
    assert "first heading is not a numbered" in errors[0]


def test_reports_adr_serial_and_slug_mismatch(tmp_path: Path) -> None:
    adrs_root = tmp_path / "docs" / "adrs"
    adrs_root.mkdir(parents=True)
    (adrs_root / "adr-0001-wrong-slug.md").write_text(
        "# Architectural decision record (ADR) 0002: Canonical title\n",
        encoding="utf-8",
    )

    errors = validate_record_files(PathRecordReader(), tmp_path / "docs")

    assert any(
        "filename serial 0001 disagrees with heading serial 0002" in error
        for error in errors
    )
    assert any("should be 'canonical-title'" in error for error in errors)


def test_markdown_file_scan_prunes_excluded_directories(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    docs_root = tmp_path / "docs"
    docs_root.mkdir()
    contents = docs_root / "contents.md"
    contents.write_text("", encoding="utf-8")
    excluded_directories = {
        ".git",
        ".uv-cache",
        ".uv-tools",
        ".venv",
        "dist",
        "node_modules",
        "target",
    }
    for excluded in excluded_directories:
        generated = tmp_path / excluded
        generated.mkdir()
        (generated / "generated.md").write_text("", encoding="utf-8")
    linked_file = docs_root / "linked.md"
    linked_file.symlink_to(contents)
    linked_directory = tmp_path / "linked-directory"
    linked_directory.symlink_to(docs_root, target_is_directory=True)

    visited: list[Path] = []
    original_iterdir = Path.iterdir

    def track_directory_visits(directory: Path):
        visited.append(directory)
        return original_iterdir(directory)

    monkeypatch.setattr(Path, "iterdir", track_directory_visits)

    assert markdown_files(tmp_path) == [contents, linked_file]
    assert not excluded_directories.intersection(path.name for path in visited)
    assert linked_directory not in visited


def test_local_record_links_handle_encoded_root_external_and_escape_paths(
    tmp_path: Path,
) -> None:
    docs_root = tmp_path / "docs"
    adrs_root = docs_root / "adrs"
    adrs_root.mkdir(parents=True)
    (adrs_root / "adr-0001-example.md").write_text(
        "# Architectural decision record (ADR) 0001: Example\n", encoding="utf-8"
    )
    source = docs_root / "contents.md"
    source.write_text(
        "[root](/docs/adrs/adr-0001-example.md?view=full#decision)\n"
        "[encoded](adrs/adr-0001%2Dexample.md)\n"
        "[external](https://example.test/adrs/adr-9999-missing.md)\n"
        "[escaping](../../outside/adrs/adr-0002-outside.md)\n",
        encoding="utf-8",
    )

    errors, checked_links = validate_local_record_links(
        PathRecordReader(), tmp_path, [source]
    )

    assert checked_links == 3
    assert len(errors) == 1
    assert "escapes the repository" in errors[0]


def test_checks_filename_heading_slug_and_record_links(tmp_path: Path) -> None:
    docs_root = tmp_path / "docs"
    adrs_root = docs_root / "adrs"
    rfcs_root = docs_root / "rfcs"
    adrs_root.mkdir(parents=True)
    rfcs_root.mkdir()
    record = adrs_root / "adr-0001-cafe-and-aether.md"
    record.write_text(
        "# Architectural decision record (ADR) 0001: Café and Æther\n"
        "[related decision](adr-0002-related-decision.md)\n",
        encoding="utf-8",
    )
    related_record = adrs_root / "adr-0002-related-decision.md"
    related_record.write_text(
        "# Architectural decision record (ADR) 0002: Related decision\n",
        encoding="utf-8",
    )
    rfc = rfcs_root / "rfc-0001-example-proposal.md"
    rfc.write_text("# RFC 0001: Example proposal\n", encoding="utf-8")
    source = docs_root / "contents.md"
    source.write_text(
        "[record](adrs/adr-0001-cafe-and-aether.md)\n"
        "[proposal](rfcs/rfc-0001-example-proposal.md)\n"
        "[RFC index](rfcs/)\n"
        "[^2]: `ADR-0001-3` (docs/adrs/missing-record.md\n",
        encoding="utf-8",
    )

    assert validate_record_files(PathRecordReader(), docs_root) == []
    assert validate_local_record_links(
        PathRecordReader(), tmp_path, [source, record]
    ) == ([], 3)


@pytest.mark.parametrize(
    "destination",
    ("adrs/adr-0001-retired-name.md", "rfcs/missing-record.md"),
)
def test_reports_missing_record_link(tmp_path: Path, destination: str) -> None:
    docs_root = tmp_path / "docs"
    (docs_root / "adrs").mkdir(parents=True)
    (docs_root / "rfcs").mkdir()
    source = docs_root / "contents.md"
    source.write_text(f"[record]({destination})\n", encoding="utf-8")

    errors, checked_links = validate_local_record_links(
        PathRecordReader(), tmp_path, [source]
    )

    assert checked_links == 1
    assert len(errors) == 1


def test_validates_html_record_links(tmp_path: Path) -> None:
    docs_root = tmp_path / "docs"
    adrs_root = docs_root / "adrs"
    adrs_root.mkdir(parents=True)
    (adrs_root / "adr-0001-example.md").write_text(
        "# Architectural decision record (ADR) 0001: Example\n", encoding="utf-8"
    )
    source = docs_root / "contents.md"
    source.write_text(
        '<a href="adrs/adr-0001-example.md">present</a>\n'
        '<a href="adrs/adr-0002-missing.md">missing</a>\n',
        encoding="utf-8",
    )

    errors, checked_links = validate_local_record_links(
        PathRecordReader(), tmp_path, [source]
    )

    assert checked_links == 2
    assert len(errors) == 1
    assert "adr-0002-missing.md" in errors[0]


def test_ignores_malformed_external_url(tmp_path: Path) -> None:
    docs_root = tmp_path / "docs"
    docs_root.mkdir()
    source = docs_root / "contents.md"
    source.write_text("[broken](http://[abc)\n", encoding="utf-8")

    errors, checked_links = validate_local_record_links(
        PathRecordReader(), tmp_path, [source]
    )

    assert (errors, checked_links) == ([], 0)


def test_reports_local_path_with_null_byte_as_diagnostic(tmp_path: Path) -> None:
    docs_root = tmp_path / "docs"
    (docs_root / "adrs").mkdir(parents=True)
    source = docs_root / "contents.md"
    source.write_text("[broken](adrs/adr-0001%00.md)\n", encoding="utf-8")

    errors, checked_links = validate_local_record_links(
        PathRecordReader(), tmp_path, [source]
    )

    assert checked_links == 1
    assert len(errors) == 1
    assert "malformed local record link" in errors[0]


class _StubReader:
    """Serve record text and link resolution from memory."""

    def __init__(
        self,
        records: dict[Path, str],
        resolved_links: dict[str, Path] | None = None,
    ) -> None:
        self._records = records
        self._resolved_links = resolved_links or {}

    def record_files(self, directory: Path) -> list[Path]:
        return sorted(path for path in self._records if path.parent == directory)

    def markdown_paths(self, repository_root: Path) -> list[Path]:
        return sorted(self._records)

    def read_text(self, path: Path) -> str:
        return self._records[path]

    def resolve_link(
        self, repository_root: Path, source: Path, link_path: str
    ) -> Path | None:
        return self._resolved_links.get(link_path)

    def is_file(self, path: Path) -> bool:
        return path in self._records


def test_validation_uses_injected_reader_without_filesystem() -> None:
    docs_root = Path("/nonexistent/docs")
    record = docs_root / "adrs" / "adr-0001-example.md"
    reader = _StubReader(
        {record: "# Architectural decision record (ADR) 0001: Example\n"}
    )

    assert validate_record_files(reader, docs_root) == []
    assert validate_local_record_links(reader, Path("/nonexistent"), [record]) == (
        [],
        0,
    )


def test_stub_reader_resolves_local_record_links_without_filesystem() -> None:
    repository_root = Path("/nonexistent")
    docs_root = repository_root / "docs"
    source = docs_root / "contents.md"
    record = docs_root / "adrs" / "adr-0001-example.md"
    reader = _StubReader(
        {
            source: "[record](adrs/adr-0001-example.md)\n",
            record: "# Architectural decision record (ADR) 0001: Example\n",
        },
        {"adrs/adr-0001-example.md": record},
    )

    assert validate_local_record_links(reader, repository_root, [source]) == ([], 1)
