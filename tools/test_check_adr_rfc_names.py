"""Tests for the ADR/RFC documentation contract checker."""

from __future__ import annotations

import ast
from pathlib import Path

import adr_rfc_names
import check_adr_rfc_names
import pytest

from adr_rfc_names import (
    LinkFacts,
    MarkdownDocument,
    RecordFile,
    extract_markdown_destinations,
    slug_for_title,
    validate_local_record_links,
    validate_record_files,
)
from adr_rfc_repository import PathRecordReader, markdown_files


def test_slug_transliterates_accents_and_keeps_kebab_case() -> None:
    assert slug_for_title("Café and Æther") == "cafe-and-aether", (
        "accents and ligatures must transliterate"
    )


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
    ], "inline links, reference definitions, and HTML links must be extracted"


def test_extracts_wrapped_link_text_and_destination() -> None:
    markdown = (
        "- [ADR-0002: Library-first internationalization and localization with\n"
        "  Fluent](\n"
        "adrs/adr-0002-library-first-internationalization-and-localization-with-fluent.md\n"
        ")\n"
    )

    assert extract_markdown_destinations(markdown) == [
        "adrs/adr-0002-library-first-internationalization-and-localization-with-fluent.md"
    ], "link text and destination may wrap across lines"


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
    ], "an info string must not close the enclosing fence"


def test_footnote_continuations_are_not_reference_links() -> None:
    markdown = """
[^2]: `ADR-0001-3`
      (docs/adrs/adr-0001-example.md decision 3)
"""

    assert extract_markdown_destinations(markdown) == [], (
        "footnote continuations must not be read as links"
    )


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

    errors = validate_record_files(PathRecordReader(tmp_path))

    assert len(errors) == 1, errors
    assert "duplicate ADR serial 0001" in errors[0], errors


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

    errors = validate_record_files(PathRecordReader(tmp_path))

    assert sum("filename is not canonical" in error for error in errors) == 2, errors
    assert any("duplicate RFC serial 0001" in error for error in errors), errors
    assert any("disagrees with heading serial 0004" in error for error in errors), (
        errors
    )
    assert any("should be 'canonical-title'" in error for error in errors), errors


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

    errors = validate_record_files(PathRecordReader(tmp_path))

    assert len(errors) == 1, errors
    assert "first heading is not a numbered" in errors[0], errors


def test_reports_adr_serial_and_slug_mismatch(tmp_path: Path) -> None:
    adrs_root = tmp_path / "docs" / "adrs"
    adrs_root.mkdir(parents=True)
    (adrs_root / "adr-0001-wrong-slug.md").write_text(
        "# Architectural decision record (ADR) 0002: Canonical title\n",
        encoding="utf-8",
    )

    errors = validate_record_files(PathRecordReader(tmp_path))

    assert any(
        "filename serial 0001 disagrees with heading serial 0002" in error
        for error in errors
    ), errors
    assert any("should be 'canonical-title'" in error for error in errors), errors


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

    assert markdown_files(tmp_path) == [contents, linked_file], (
        "only the contents file and its symlink must be discovered"
    )
    assert not excluded_directories.intersection(
        path.name for path in visited
    ), "excluded directories must not be scanned"
    assert linked_directory not in visited, (
        "a symlinked directory must not be followed"
    )


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

    errors, checked_links = validate_local_record_links(PathRecordReader(tmp_path))

    assert checked_links == 3, "root, encoded, and escaping links must be checked"
    assert len(errors) == 1, errors
    assert "escapes the repository" in errors[0], errors


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

    assert validate_record_files(PathRecordReader(tmp_path)) == [], (
        "a canonical record set must produce no errors"
    )
    assert validate_local_record_links(PathRecordReader(tmp_path)) == ([], 3), (
        "every visible local record link must be checked"
    )


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

    errors, checked_links = validate_local_record_links(PathRecordReader(tmp_path))

    assert checked_links == 1, "the single local link must be checked"
    assert len(errors) == 1, errors


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

    errors, checked_links = validate_local_record_links(PathRecordReader(tmp_path))

    assert checked_links == 2, "both HTML links must be checked"
    assert len(errors) == 1, errors
    assert "adr-0002-missing.md" in errors[0], errors


def test_ignores_malformed_external_url(tmp_path: Path) -> None:
    docs_root = tmp_path / "docs"
    docs_root.mkdir()
    source = docs_root / "contents.md"
    source.write_text("[broken](http://[abc)\n", encoding="utf-8")

    errors, checked_links = validate_local_record_links(PathRecordReader(tmp_path))

    assert (errors, checked_links) == ([], 0), (
        "a malformed external URL must be ignored"
    )


def test_reports_local_path_with_null_byte_as_diagnostic(tmp_path: Path) -> None:
    docs_root = tmp_path / "docs"
    (docs_root / "adrs").mkdir(parents=True)
    source = docs_root / "contents.md"
    source.write_text("[broken](adrs/adr-0001%00.md)\n", encoding="utf-8")

    errors, checked_links = validate_local_record_links(PathRecordReader(tmp_path))

    assert checked_links == 1, "the malformed link must still be counted"
    assert len(errors) == 1, errors
    assert "malformed local record link" in errors[0], errors


def _create_valid_records(real_root: Path) -> Path:
    """Create a record set with one valid link and return its docs directory."""
    docs_root = real_root / "docs"
    adrs_root = docs_root / "adrs"
    adrs_root.mkdir(parents=True)
    (adrs_root / "adr-0001-example.md").write_text(
        "# Architectural decision record (ADR) 0001: Example\n", encoding="utf-8"
    )
    (docs_root / "contents.md").write_text(
        "[record](adrs/adr-0001-example.md)\n", encoding="utf-8"
    )
    return docs_root


def test_checks_links_through_a_symlinked_repository_root(tmp_path: Path) -> None:
    """Containment must use the resolved root, not the path the caller supplied."""
    real_root = tmp_path / "real"
    docs_root = _create_valid_records(real_root)
    outside_root = tmp_path / "outside" / "adrs"
    outside_root.mkdir(parents=True)
    (outside_root / "adr-0002-outside.md").write_text(
        "# Architectural decision record (ADR) 0002: Outside\n", encoding="utf-8"
    )
    symlinked_root = tmp_path / "linked"
    symlinked_root.symlink_to(real_root, target_is_directory=True)

    reader = PathRecordReader(symlinked_root)

    assert reader.root == real_root.resolve(), "the reader must resolve its root"
    assert validate_record_files(reader) == [], (
        "records under a symlinked root must satisfy the filename contract"
    )
    assert validate_local_record_links(reader) == ([], 1), (
        "a valid link reached through a symlinked root must not be reported"
    )

    (docs_root / "contents.md").write_text(
        "[record](adrs/adr-0001-example.md)\n"
        "[escape](../../outside/adrs/adr-0002-outside.md)\n",
        encoding="utf-8",
    )

    errors, checked_links = validate_local_record_links(
        PathRecordReader(symlinked_root)
    )

    assert checked_links == 2, "both links through the symlinked root are checked"
    assert len(errors) == 1, errors
    assert "escapes the repository" in errors[0], errors


def test_reports_root_resolution_failure_before_any_discovery(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """A root that cannot resolve must become a read error, not a check run."""

    def fail_to_resolve(*_args: object, **_kwargs: object) -> Path:
        raise OSError("symlink loop")

    def forbidden_discovery(*_args: object, **_kwargs: object) -> list[str]:
        raise AssertionError("discovery must not run for an unresolvable root")

    monkeypatch.setattr(Path, "resolve", fail_to_resolve)
    monkeypatch.setattr(
        check_adr_rfc_names, "validate_record_files", forbidden_discovery
    )

    check = check_adr_rfc_names.check_documentation(tmp_path)

    assert check.read_error == "symlink loop", check.read_error
    assert (check.errors, check.records, check.checked_links) == ((), 0, 0), check


class _StubReader:
    """Serve record text and link facts from memory."""

    def __init__(
        self,
        records: list[RecordFile] | None = None,
        documents: list[MarkdownDocument] | None = None,
        facts: dict[str, LinkFacts | None] | None = None,
    ) -> None:
        self._records = records or []
        self._documents = documents or []
        self._facts = facts or {}

    def record_files(self, family: str) -> list[RecordFile]:
        prefix = f"docs/{family}/"
        return [
            record
            for record in self._records
            if record.relative_path.startswith(prefix)
        ]

    def markdown_documents(self) -> list[MarkdownDocument]:
        return list(self._documents)

    def link_facts(self, source: str, destination_path: str) -> LinkFacts | None:
        return self._facts.get(destination_path)


def test_validation_uses_injected_reader_without_filesystem() -> None:
    reader = _StubReader(
        records=[
            RecordFile(
                "adr-0001-example.md",
                "docs/adrs/adr-0001-example.md",
                "# Architectural decision record (ADR) 0001: Example\n",
            )
        ]
    )

    assert validate_record_files(reader) == [], (
        "validation must read records through the injected reader"
    )
    assert validate_local_record_links(reader) == ([], 0), (
        "validation must read documents through the injected reader"
    )


def test_stub_reader_supplies_local_record_link_facts() -> None:
    reader = _StubReader(
        documents=[
            MarkdownDocument(
                "docs/contents.md", "[record](adrs/adr-0001-example.md)\n"
            )
        ],
        facts={"adrs/adr-0001-example.md": LinkFacts(inside_root=True, exists=True)},
    )

    assert validate_local_record_links(reader) == ([], 1), (
        "the stub reader must supply link facts without a filesystem"
    )


def test_rules_reject_a_link_the_adapter_reports_outside_the_root() -> None:
    reader = _StubReader(
        documents=[
            MarkdownDocument(
                "docs/contents.md",
                "[escape](../../outside/adrs/adr-0002-outside.md)\n",
            )
        ],
        facts={
            "../../outside/adrs/adr-0002-outside.md": LinkFacts(
                inside_root=False, exists=False
            )
        },
    )

    errors, checked_links = validate_local_record_links(reader)

    assert checked_links == 1, "the escaping link must still be counted"
    assert len(errors) == 1, errors
    assert "escapes the repository" in errors[0], errors


def test_rules_reject_a_link_the_adapter_reports_missing() -> None:
    reader = _StubReader(
        documents=[
            MarkdownDocument(
                "docs/contents.md", "[record](adrs/adr-0009-missing.md)\n"
            )
        ],
        facts={"adrs/adr-0009-missing.md": LinkFacts(inside_root=True, exists=False)},
    )

    errors, checked_links = validate_local_record_links(reader)

    assert checked_links == 1, "the missing link must still be counted"
    assert len(errors) == 1, errors
    assert "does not exist" in errors[0], errors


FILESYSTEM_METHODS = (
    "resolve",
    "read_text",
    "iterdir",
    "glob",
    "exists",
    "is_dir",
    "is_file",
    "is_symlink",
)


def _direct_filesystem_calls(tree: ast.AST) -> list[str]:
    """Return filesystem calls that bypass the injected `reader` parameter."""
    calls: list[str] = []
    for node in ast.walk(tree):
        match node:
            case ast.Call(func=ast.Attribute(value=ast.Name(id="reader"))):
                continue
            case ast.Call(func=ast.Attribute(attr=attr)) if (
                attr in FILESYSTEM_METHODS
            ):
                calls.append(attr)
            case ast.Call(func=ast.Name(id="open")):
                calls.append("open")
    return sorted(calls)


def test_rules_module_has_no_direct_filesystem_calls() -> None:
    source = Path(adr_rfc_names.__file__).read_text(encoding="utf-8")
    tree = ast.parse(source)

    assert _direct_filesystem_calls(tree) == [], (
        "the rules module must ask the adapter for link facts instead of "
        "touching the filesystem"
    )


def test_rules_module_imports_no_filesystem_path_type() -> None:
    source = Path(adr_rfc_names.__file__).read_text(encoding="utf-8")
    tree = ast.parse(source)
    imported_modules = {
        node.module
        for node in ast.walk(tree)
        if isinstance(node, ast.ImportFrom) and node.module
    }
    imported_modules.update(
        alias.name
        for node in ast.walk(tree)
        if isinstance(node, ast.Import)
        for alias in node.names
    )

    assert "pathlib" not in imported_modules, (
        "the rules must model repository paths as strings supplied by the port"
    )
