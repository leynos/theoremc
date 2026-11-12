"""End-to-end and Makefile contract tests for the ADR/RFC checker command."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

import check_adr_rfc_names
import pytest


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
        "adr_rfc_ports.py",
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
    assert "Checked 2 ADR/RFC records and 2 local ADR/RFC links." in result.stdout, (
        result.stdout
    )


def test_command_reports_contract_errors(tmp_path: Path) -> None:
    checker = _create_repository(tmp_path)
    docs_root = tmp_path / "docs"
    (docs_root / "adrs" / "adr-1-noncanonical.md").write_text("", encoding="utf-8")
    (docs_root / "contents.md").write_text(
        "[missing](adrs/adr-0009-missing.md)\n", encoding="utf-8"
    )

    result = _run_checker(checker)

    assert result.returncode == 1, result.stderr
    assert "filename is not canonical" in result.stderr, result.stderr
    assert "local ADR/RFC link target does not exist" in result.stderr, result.stderr


def test_command_reports_invalid_utf8_without_traceback(tmp_path: Path) -> None:
    checker = _create_repository(tmp_path)
    invalid_record = tmp_path / "docs" / "adrs" / "adr-0002-invalid.md"
    invalid_record.write_bytes(b"\xff")

    result = _run_checker(checker)

    assert result.returncode == 1, result.stderr
    assert "could not read repository files" in result.stderr, result.stderr
    assert "Traceback" not in result.stderr, result.stderr


def test_command_reports_malformed_destination_without_traceback(
    tmp_path: Path,
) -> None:
    checker = _create_repository(tmp_path)
    (tmp_path / "docs" / "contents.md").write_text(
        "[broken](adrs/adr-0001%00.md)\n[url](http://[abc)\n",
        encoding="utf-8",
    )

    result = _run_checker(checker)

    assert result.returncode == 1, result.stderr
    assert "malformed local record link" in result.stderr, result.stderr
    assert "Traceback" not in result.stderr, result.stderr


def test_command_reports_filesystem_failure_without_traceback(
    tmp_path: Path,
) -> None:
    checker = _create_repository(tmp_path)
    (tmp_path / "docs" / "broken.md").symlink_to(tmp_path / "missing.md")

    result = _run_checker(checker)

    assert result.returncode == 1, result.stderr
    assert "could not read repository files" in result.stderr, result.stderr
    assert "Traceback" not in result.stderr, result.stderr


def test_main_reports_symlink_loop_root_without_traceback(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    loop_root = tmp_path / "loop"
    loop_root.symlink_to(loop_root)
    monkeypatch.setattr(check_adr_rfc_names, "REPOSITORY_ROOT", loop_root)

    assert check_adr_rfc_names.main() == 1, "an unresolvable root must fail"

    captured = capsys.readouterr()
    assert "could not read repository files" in captured.err, captured.err
    assert "Traceback" not in captured.err, captured.err


def test_make_target_executes_both_checks_with_pinned_interpreter(
    tmp_path: Path,
) -> None:
    repository_root = Path(__file__).resolve().parents[1]
    log = tmp_path / "uv-calls.log"
    stub = tmp_path / "uv"
    stub.write_text(
        '#!/bin/sh\nprintf \'%s\\n\' "$*" >> "$UV_LOG"\n',
        encoding="utf-8",
    )
    stub.chmod(0o755)
    environment = {**os.environ, "UV_LOG": str(log)}

    result = subprocess.run(
        ["make", f"UV={stub}", "check-adr-rfc-names"],
        cwd=repository_root,
        capture_output=True,
        text=True,
        check=False,
        env=environment,
    )

    assert result.returncode == 0, result.stderr
    commands = log.read_text(encoding="utf-8").splitlines()
    assert len(commands) == 2, commands
    assert all("--python 3.13" in command for command in commands), commands
    assert any(
        "-m pytest tools/test_check_adr_rfc_names.py" in command
        for command in commands
    ), commands
    assert any(
        command.endswith("tools/check_adr_rfc_names.py") for command in commands
    ), commands
