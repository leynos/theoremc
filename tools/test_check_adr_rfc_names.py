"""Unit tests for the ADR/RFC documentation contract checker."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from check_adr_rfc_names import (
    extract_markdown_destinations,
    slug_for_title,
    validate_local_record_links,
    validate_record_files,
)


class SlugTests(unittest.TestCase):
    def test_transliterates_accents_and_keeps_kebab_case(self) -> None:
        self.assertEqual(slug_for_title("Café and Æther"), "cafe-and-aether")


class MarkdownDestinationTests(unittest.TestCase):
    def test_extracts_local_links_but_ignores_code(self) -> None:
        markdown = """
[record](adrs/adr-0001-example.md#decision)
[reference]: <rfcs/rfc-0001-example.md>
`[inline code](adrs/ignored.md)`

```markdown
[fenced code](adrs/also-ignored.md)
```
"""
        self.assertEqual(
            extract_markdown_destinations(markdown),
            ["adrs/adr-0001-example.md#decision", "rfcs/rfc-0001-example.md"],
        )

    def test_footnote_continuations_are_not_reference_links(self) -> None:
        markdown = """
[^2]: `ADR-0001-3`
      (docs/adrs/adr-0001-example.md decision 3)
"""
        self.assertEqual(extract_markdown_destinations(markdown), [])


class RepositoryContractTests(unittest.TestCase):
    def test_reports_duplicate_record_serials(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            docs_root = Path(temporary_directory) / "docs"
            adrs_root = docs_root / "adrs"
            adrs_root.mkdir(parents=True)
            (adrs_root / "adr-0001-first-decision.md").write_text(
                "# Architectural decision record (ADR) 0001: First decision\n",
                encoding="utf-8",
            )
            (adrs_root / "adr-0001-second-decision.md").write_text(
                "# Architectural decision record (ADR) 0001: Second decision\n",
                encoding="utf-8",
            )

            errors = validate_record_files(docs_root)

            self.assertEqual(len(errors), 1)
            self.assertIn("duplicate ADR serial 0001", errors[0])

    def test_reports_heading_serial_and_title_slug_mismatches(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            docs_root = Path(temporary_directory) / "docs"
            adrs_root = docs_root / "adrs"
            adrs_root.mkdir(parents=True)
            (adrs_root / "adr-0001-wrong-name.md").write_text(
                "# Architectural decision record (ADR) 0002: Canonical title\n",
                encoding="utf-8",
            )

            errors = validate_record_files(docs_root)

            self.assertEqual(len(errors), 2)
            self.assertTrue(any("disagrees with heading serial 0002" in error for error in errors))
            self.assertTrue(any("should be 'canonical-title'" in error for error in errors))

    def test_checks_filename_heading_slug_and_record_links(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository_root = Path(temporary_directory)
            docs_root = repository_root / "docs"
            adrs_root = docs_root / "adrs"
            adrs_root.mkdir(parents=True)
            rfcs_root = docs_root / "rfcs"
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

            self.assertEqual(validate_record_files(docs_root), [])
            self.assertEqual(
                validate_local_record_links(repository_root, [source, record]),
                ([], 3),
            )

            source.write_text("[record](adrs/adr-0001-retired-name.md)\n", encoding="utf-8")
            errors, checked_links = validate_local_record_links(repository_root, [source])
            self.assertEqual(checked_links, 1)
            self.assertEqual(len(errors), 1)

            source.write_text("[record](rfcs/missing-record.md)\n", encoding="utf-8")
            errors, checked_links = validate_local_record_links(repository_root, [source])
            self.assertEqual(checked_links, 1)
            self.assertEqual(len(errors), 1)


if __name__ == "__main__":
    unittest.main()
