#!/usr/bin/env python3
"""Command-line entry point for the ADR/RFC documentation contract checker.

Run it with ``python3 tools/check_adr_rfc_names.py``, or run
``make check-adr-rfc-names`` to execute this checker after its pytest suite.
The validation rules live in ``tools/adr_rfc_names.py`` and the filesystem
adapter in ``tools/adr_rfc_repository.py``.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path

from adr_rfc_names import (
    RECORD_KINDS,
    validate_local_record_links,
    validate_record_files,
)
from adr_rfc_repository import PathRecordReader


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class DocumentationCheck:
    """Outcome of the ADR/RFC documentation checks.

    Example: a successful check has no errors and a `None` read error.
    """

    errors: tuple[str, ...]
    records: int
    checked_links: int
    read_error: str | None


def check_documentation(repository_root: Path) -> DocumentationCheck:
    """Run every ADR/RFC check, reporting read failures as a value.

    The adapter is built inside the guarded region: resolving the repository
    root can fail, and that failure must become a `read_error` before any
    discovery or link containment runs.

    Example: an unreadable record returns a `DocumentationCheck` whose
    `read_error` describes the failure instead of raising.
    """
    try:
        reader = PathRecordReader(repository_root)
        errors = validate_record_files(reader)
        link_errors, checked_links = validate_local_record_links(reader)
        records = sum(
            len(reader.record_files(record_kind.family))
            for record_kind in RECORD_KINDS
        )
    except (OSError, RuntimeError, UnicodeError) as error:
        return DocumentationCheck((), 0, 0, str(error))
    return DocumentationCheck(tuple(errors + link_errors), records, checked_links, None)


def main() -> int:
    """Run the repository's ADR/RFC filename and inbound-link checks.

    Example: `main()` returns `0` when every record and local link is valid.
    """
    check = check_documentation(REPOSITORY_ROOT)
    if check.read_error is not None:
        print(
            "ADR/RFC documentation check could not read repository files: "
            f"{check.read_error}",
            file=sys.stderr,
        )
        return 1
    if check.errors:
        print("ADR/RFC documentation check failed:", file=sys.stderr)
        for error in check.errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print(
        f"Checked {check.records} ADR/RFC records and "
        f"{check.checked_links} local ADR/RFC links."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())