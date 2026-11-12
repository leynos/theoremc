"""Filesystem adapter for the ADR/RFC documentation checks.

The validation rules in ``adr_rfc_names.py`` depend only on the narrow
``RecordReader`` port and their own domain values. This module supplies the
filesystem implementation used by the command-line entry point, so discovery,
reads, and link containment stay outside the rules.
"""

from __future__ import annotations

from pathlib import Path
from urllib.parse import unquote

from adr_rfc_ports import LinkFacts, MarkdownDocument, RecordFile, RecordReader


class SymlinkedRecordDirectoryError(Exception):
    """Report a record directory that is reached through a symbolic link.

    ``markdown_files`` does not follow symlinked directories, so records read
    through one would not be the files link validation inspects. Rejecting the
    directory keeps both checks on one set of files.

    Example: `docs/adrs` pointing outside the repository raises this error
    rather than importing external records.
    """


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


def _reject_symlinked_record_directories(root: Path, directory: Path) -> None:
    """Reject a record directory reached through any symbolic link.

    Every component between the repository root and the record directory is
    checked, because a symlinked `docs` escapes containment exactly as a
    symlinked `docs/adrs` does.

    Example: `docs/adrs -> /outside/adrs` makes `record_files("adrs")` raise
    `SymlinkedRecordDirectoryError`.
    """
    candidate = root
    for part in directory.relative_to(root).parts:
        candidate = candidate / part
        if candidate.is_symlink():
            raise SymlinkedRecordDirectoryError(
                f"{candidate.relative_to(root).as_posix()}: record directory "
                "must not be a symbolic link"
            )


def _markdown_candidates(directory: Path) -> list[Path]:
    """Return a record directory's Markdown entries, in any extension case.

    Discovery is case-insensitive so that a record named
    `adr-0001-example.MD` reaches the canonical filename check instead of
    being skipped; that check then reports the wrong casing.

    Example: `adr-0001-example.MD` is returned, while `notes.txt` is not.
    """
    return sorted(
        path for path in directory.iterdir() if path.suffix.lower() == ".md"
    )


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

        Parameters
        ----------
        family : str
            Record family directory name under `docs/`, such as `"adrs"`.

        Returns
        -------
        list[RecordFile]
            Every Markdown entry in the directory, read as UTF-8. A missing
            directory yields none.

        Raises
        ------
        SymlinkedRecordDirectoryError
            If the record directory, or any directory between it and the
            repository root, is a symbolic link.

        Example: `record_files("adrs")` lists every ADR with its text.
        """
        directory = self._root / "docs" / family
        _reject_symlinked_record_directories(self._root, directory)
        if not directory.is_dir():
            return []
        return [
            RecordFile(
                path.name,
                self._relative_path(path),
                path.read_text(encoding="utf-8"),
            )
            for path in _markdown_candidates(directory)
        ]

    def markdown_documents(self) -> list[MarkdownDocument]:
        """Return every repository Markdown document to inspect.

        Returns
        -------
        list[MarkdownDocument]
            Every Markdown file outside generated directories, read as UTF-8
            and ordered by path.

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
        """Describe a local link target, resolving it from the right directory.

        Parameters
        ----------
        source : str
            Repository-relative path of the document that holds the link.
        destination_path : str
            Link destination as written in the document. A leading `/` is
            repository-relative; anything else is relative to the directory
            that holds `source`.

        Returns
        -------
        LinkFacts | None
            Whether the resolved target is inside the repository and exists,
            or `None` when the destination is not an interpretable path.

        Example: a root-relative `/docs/adrs/adr-0001-example.md` resolves from
        the repository root, while `adrs/adr-0001-example.md` resolves from the
        source document's directory, so `docs/contents.md` reaches
        `docs/adrs/adr-0001-example.md`.
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