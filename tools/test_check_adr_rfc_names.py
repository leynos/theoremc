"""Tests for the ADR/RFC documentation contract checker."""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import pytest

from check_adr_rfc_names import (
    extract_markdown_destinations,
    markdown_files,
    slug_for_title,
    validate_local_record_links,
    validate_record_files,
)


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

    errors = validate_record_files(tmp_path / "docs")

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

    errors = validate_record_files(tmp_path / "docs")

    assert sum("filename is not canonical" in error for error in errors) == 2
    assert any("duplicate RFC serial 0001" in error for error in errors)
    assert any("disagrees with heading serial 0004" in error for error in errors)
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

    errors, checked_links = validate_local_record_links(tmp_path, [source])

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

    assert validate_record_files(docs_root) == []
    assert validate_local_record_links(tmp_path, [source, record]) == ([], 3)


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

    errors, checked_links = validate_local_record_links(tmp_path, [source])

    assert checked_links == 1
    assert len(errors) == 1


def _create_repository(root: Path) -> Path:
    tools_root = root / "tools"
    docs_root = root / "docs"
    (docs_root / "adrs").mkdir(parents=True)
    (docs_root / "rfcs").mkdir()
    tools_root.mkdir()
    checker = tools_root / "check_adr_rfc_names.py"
    shutil.copyfile(Path(__file__).with_name("check_adr_rfc_names.py"), checker)
    (docs_root / "adrs" / "adr-0001-example.md").write_text(
        "# Architectural decision record (ADR) 0001: Example\n", encoding="utf-8"
    )
    (docs_root / "rfcs" / "rfc-0001-proposal.md").write_text(
        "# RFC 0001: Proposal\n", encoding="utf-8"
    )
    (docs_root / "contents.md").write_text(
        "[ADR](adrs/adr-0001-example.md)\n[RFC](rfcs/rfc-0001-proposal.md)\n",
        encoding="utf-8",
    )
    return checker


def test_command_succeeds_for_valid_records_and_links(tmp_path: Path) -> None:
    checker = _create_repository(tmp_path)

    result = subprocess.run(
        [sys.executable, str(checker)], capture_output=True, text=True, check=False
    )

    assert result.returncode == 0, result.stderr
    assert "Checked 2 ADR/RFC records and 2 local ADR/RFC links." in result.stdout


def test_command_reports_contract_errors(tmp_path: Path) -> None:
    checker = _create_repository(tmp_path)
    docs_root = tmp_path / "docs"
    (docs_root / "adrs" / "adr-1-noncanonical.md").write_text("", encoding="utf-8")
    (docs_root / "contents.md").write_text(
        "[missing](adrs/adr-0009-missing.md)\n", encoding="utf-8"
    )

    result = subprocess.run(
        [sys.executable, str(checker)], capture_output=True, text=True, check=False
    )

    assert result.returncode == 1
    assert "filename is not canonical" in result.stderr
    assert "local ADR/RFC link target does not exist" in result.stderr


def test_command_reports_invalid_utf8_without_traceback(tmp_path: Path) -> None:
    checker = _create_repository(tmp_path)
    invalid_record = tmp_path / "docs" / "adrs" / "adr-0002-invalid.md"
    invalid_record.write_bytes(b"\xff")

    result = subprocess.run(
        [sys.executable, str(checker)], capture_output=True, text=True, check=False
    )

    assert result.returncode == 1
    assert "could not read repository files" in result.stderr
    assert "Traceback" not in result.stderr
