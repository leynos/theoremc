"""Filesystem adapter for the ADR/RFC documentation checks.

The validation rules in ``adr_rfc_names.py`` depend only on the narrow
``RecordReader`` port and their own domain values. This module supplies the
filesystem implementation used by the command-line entry point, so discovery,
reads, and link containment stay outside the rules.
"""

from __future__ import annotations

from pathlib import Path
from urllib.parse import unquote

from adr_rfc_names import LinkFacts, MarkdownDocument, RecordFile, RecordReader


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


def _is_inside(root: Path, target: Path) -> bool:
    """Return whether a resolved target still sits under the repository root.

    Example: a link resolving to `/repo/../outside/adr.md` is not inside
    `/repo`.
    """
    try:
        target.relative_to(root)
    except ValueError:
        return False
    return True


class PathRecordReader(RecordReader):
    """Read ADR/RFC inputs from the local filesystem.

    The constructor resolves the repository root strictly and once, so
    discovery, reads, and link containment all compare against the same real
    path. Strict resolution is deliberate: without it a missing root or a
    symlink loop resolves to the unresolved path, and the checks would report
    on a directory that cannot be read. The failure propagates instead, and the
    composition root reports it as a read error.
    """

    def __init__(self, repository_root: Path) -> None:
        self._root = repository_root.resolve(strict=True)

    @property
    def root(self) -> Path:
        """Return the resolved repository root.

        Example: a reader built from a symlinked path reports the real path.
        """
        return self._root

    def _relative_path(self, path: Path) -> str:
        """Return a repository relative Posix path for diagnostics.

        Example: `_relative_path(root / "docs/contents.md")` returns
        `"docs/contents.md"`.
        """
        return path.relative_to(self._root).as_posix()

    def record_files(self, family: str) -> list[RecordFile]:
        """Return the Markdown records in a family such as `"adrs"`.

        Example: `record_files("adrs")` lists every ADR with its text.
        """
        directory = self._root / "docs" / family
        return [
            RecordFile(
                path.name,
                self._relative_path(path),
                path.read_text(encoding="utf-8"),
            )
            for path in sorted(directory.glob("*.md"))
        ]

    def markdown_documents(self) -> list[MarkdownDocument]:
        """Return every repository Markdown document to inspect.

        Example: `markdown_documents()` omits generated directories.
        """
        return [
            MarkdownDocument(
                self._relative_path(path),
                path.read_text(encoding="utf-8"),
            )
            for path in markdown_files(self._root)
        ]

    def link_facts(
        self,
        source: str,
        destination_path: str,
    ) -> LinkFacts | None:
        """Describe a local link target relative to the repository root.

        Example: a root-relative `/docs/adrs/adr-0001-example.md` resolves from
        the repository root, while `adrs/adr-0001-example.md` resolves from the
        source document's directory.
        """
        try:
            decoded_path = unquote(destination_path)
            if decoded_path.startswith("/"):
                target = (self._root / decoded_path.lstrip("/")).resolve()
            else:
                target = ((self._root / source).parent / decoded_path).resolve()
        except (OSError, RuntimeError, ValueError):
            # Encoded null bytes and unresolvable symlink chains are reported
            # by the rules as diagnostics rather than aborting the check.
            return None
        inside_root = _is_inside(self._root, target)
        return LinkFacts(
            inside_root=inside_root,
            exists=inside_root and target.is_file(),
        )