"""End-to-end and Makefile contract tests for the ADR/RFC checker command."""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path


def _create_repository(root: Path) -> Path:
    """Copy the checker modules into a fixture repository and return the CLI."""
    tools_root = root / "tools"
    docs_root = root / "docs"
    (docs_root / "adrs").mkdir(parents=True)
    (docs_root / "rfcs").mkdir()
    tools_root.mkdir()
    for module_name in (
        "check_adr_rfc_names.py",
        "adr_rfc_names.py",
        "adr_rfc_repository.py",
    ):
        shutil.copyfile(
            Path(__file__).with_name(module_name), tools_root / module_name
        )
    checker = tools_root / "check_adr_rfc_names.py"
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


def _run_checker(checker: Path) -> subprocess.CompletedProcess[str]:
    """Run the checker with the interpreter executing the test suite."""
    return subprocess.run(
        [sys.executable, str(checker)],
        capture_output=True,
        text=True,
        check=False,
    )


def test_command_succeeds_for_valid_records_and_links(tmp_path: Path) -> None:
    checker = _create_repository(tmp_path)

    result = _run_checker(checker)

    assert result.returncode == 0, result.stderr
    assert "Checked 2 ADR/RFC records and 2 local ADR/RFC links." in result.stdout


def test_command_reports_contract_errors(tmp_path: Path) -> None:
    checker = _create_repository(tmp_path)
    docs_root = tmp_path / "docs"
    (docs_root / "adrs" / "adr-1-noncanonical.md").write_text("", encoding="utf-8")
    (docs_root / "contents.md").write_text(
        "[missing](adrs/adr-0009-missing.md)\n", encoding="utf-8"
    )

    result = _run_checker(checker)

    assert result.returncode == 1
    assert "filename is not canonical" in result.stderr
    assert "local ADR/RFC link target does not exist" in result.stderr


def test_command_reports_invalid_utf8_without_traceback(tmp_path: Path) -> None:
    checker = _create_repository(tmp_path)
    invalid_record = tmp_path / "docs" / "adrs" / "adr-0002-invalid.md"
    invalid_record.write_bytes(b"\xff")

    result = _run_checker(checker)

    assert result.returncode == 1
    assert "could not read repository files" in result.stderr
    assert "Traceback" not in result.stderr


def test_command_reports_malformed_destination_without_traceback(
    tmp_path: Path,
) -> None:
    checker = _create_repository(tmp_path)
    (tmp_path / "docs" / "contents.md").write_text(
        "[broken](adrs/adr-0001%00.md)\n[url](http://[abc)\n",
        encoding="utf-8",
    )

    result = _run_checker(checker)

    assert result.returncode == 1
    assert "malformed local record link" in result.stderr
    assert "Traceback" not in result.stderr


def test_make_target_runs_both_checks_with_pinned_interpreter() -> None:
    repository_root = Path(__file__).resolve().parents[1]

    result = subprocess.run(
        ["make", "-n", "check-adr-rfc-names"],
        cwd=repository_root,
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0, result.stderr
    commands = [line for line in result.stdout.splitlines() if "python" in line]
    assert len(commands) == 2
    assert all("--python 3.13" in command for command in commands)
    assert any("tools/test_check_adr_rfc_names.py" in command for command in commands)
    assert any("tools/check_adr_rfc_names.py" in command for command in commands)
