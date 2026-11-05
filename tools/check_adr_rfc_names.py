#!/usr/bin/env python3
"""Command-line entry point for the ADR/RFC documentation contract checker.

Run it with ``python3 tools/check_adr_rfc_names.py``, or run
``make check-adr-rfc-names`` to execute this checker after its pytest suite.
The validation rules live in ``tools/adr_rfc_names.py``.
"""

from __future__ import annotations

import sys
from pathlib import Path

from adr_rfc_names import (
    markdown_files,
    validate_local_record_links,
    validate_record_files,
)


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DOCUMENTATION_ROOT = REPOSITORY_ROOT / "docs"


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
