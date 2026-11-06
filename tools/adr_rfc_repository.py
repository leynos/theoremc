"""Filesystem adapter for the ADR/RFC documentation checks.

The validation rules in ``adr_rfc_names.py`` depend only on the narrow
``RecordReader`` port. This module supplies the filesystem implementation used
by the command-line entry point, so discovery and reads stay outside the rules.
"""

from __future__ import annotations

from pathlib import Path
from urllib.parse import unquote

from adr_rfc_names import RecordReader


EXCLUDED_MARKDOWN_DIRECTORIES = frozenset(
    {".git", ".venv", "dist", "node_modules", "target", ".uv-cache", ".uv-tools"}
)


def _is_markdown_path(path: Path) -> bool:
    """Return whether a path is a Markdown file or symlink.

    Example: a symlink named `guide.md` is included in the scan.
    """
    return path.suffix == ".md" and (path.is_symlink() or path.is_file())


def _is_traversable_markdown_directory(path: Path) -> bool:
    """Return whether a real directory should be included in the scan.

    Example: a `target/` directory is not traversable.
    """
    return (
        not path.is_symlink()
        and path.is_dir()
        and path.name not in EXCLUDED_MARKDOWN_DIRECTORIES
    )


def markdown_files(repository_root: Path) -> list[Path]:
    """List repository Markdown files while excluding generated directories.

    Example: `markdown_files(Path("/repo"))` omits `/repo/target/output.md`.
    """
    markdown_paths: list[Path] = []
    directories = [repository_root]
    while directories:
        entries = tuple(directories.pop().iterdir())
        markdown_paths.extend(filter(_is_markdown_path, entries))
        directories.extend(filter(_is_traversable_markdown_directory, entries))
    return sorted(markdown_paths)


class PathRecordReader(RecordReader):
    """Read ADR/RFC inputs from the local filesystem."""

    def record_files(self, directory: Path) -> list[Path]:
        """Return the Markdown records in a directory, sorted by name.

        Example: `record_files(docs / "adrs")` lists every ADR file.
        """
        return sorted(directory.glob("*.md"))

    def markdown_paths(self, repository_root: Path) -> list[Path]:
        """Return every repository Markdown file to inspect.

        Example: `markdown_paths(Path("/repo"))` omits generated directories.
        """
        return markdown_files(repository_root)

    def read_text(self, path: Path) -> str:
        """Return a file's UTF-8 text.

        Example: `read_text(record)` returns the record's Markdown source.
        """
        return path.read_text(encoding="utf-8")

    def resolve_link(
        self,
        repository_root: Path,
        source: Path,
        link_path: str,
    ) -> Path | None:
        """Resolve a local link path, or report a malformed path as `None`.

        Example: a root-relative `/docs/adrs/adr-0001-example.md` resolves from
        the repository root.
        """
        try:
            decoded_path = unquote(link_path)
            if decoded_path.startswith("/"):
                return (repository_root / decoded_path.lstrip("/")).resolve()
            return (source.parent / decoded_path).resolve()
        except (OSError, ValueError):
            # Encoded null bytes and unresolvable symlink chains are reported
            # by the rules as diagnostics rather than aborting the check.
            return None

    def is_file(self, path: Path) -> bool:
        """Return whether a resolved link target is a regular file.

        Example: `is_file(record)` is `True` for an existing ADR.
        """
        return path.is_file()