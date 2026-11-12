"""Domain values and the driven port for the ADR/RFC documentation checks.

The rules in ``adr_rfc_names.py`` decide what is wrong with a record set; the
adapter in ``adr_rfc_repository.py`` supplies record text and filesystem
containment facts. Everything crossing the ``RecordReader`` port is a domain
value, so the rules never import a filesystem path type.

Example: a test double returns `RecordFile` values from memory, so the rules
can be exercised without a repository on disk.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol


@dataclass(frozen=True)
class RecordFile:
    """Describe one record file in terms the rules can reason about.

    The adapter reports paths relative to the repository root, so the rules
    never need to know how that root is spelled on disk.

    Attributes
    ----------
    name : str
        File name, matched against the family's canonical filename pattern.
    relative_path : str
        Repository-relative path, quoted in every diagnostic.
    text : str
        File contents, used to check the numbered heading.

    Example: `RecordFile("adr-0001-example.md",
    "docs/adrs/adr-0001-example.md", "# …")`.
    """

    name: str
    relative_path: str
    text: str


@dataclass(frozen=True)
class MarkdownDocument:
    """Describe one Markdown file whose links the rules must check.

    Attributes
    ----------
    relative_path : str
        Repository-relative path of the document, used as the link source.
    text : str
        File contents, searched for ADR/RFC link destinations.

    Example: `MarkdownDocument("docs/contents.md", "[x](adrs/adr-0001.md)")`.
    """

    relative_path: str
    text: str


@dataclass(frozen=True)
class LinkFacts:
    """Describe what the adapter established about a local record link.

    Containment is a filesystem question, so the adapter answers it and the
    rules decide whether the answer is an error.

    Attributes
    ----------
    inside_root : bool
        Whether the resolved target still sits under the repository root.
    exists : bool
        Whether the resolved target is an existing file. It is `False` when
        the target is outside the root, because that target is never inspected.

    Example: a link to a missing ADR reports `inside_root=True, exists=False`.
    """

    inside_root: bool
    exists: bool


class RecordReader(Protocol):
    """Supply the record and link facts the ADR/RFC checks need.

    Every value crossing this port is a domain value: record names, repository
    relative paths, file text, and containment facts. ``PathRecordReader`` in
    ``adr_rfc_repository.py`` implements it against the filesystem.

    Example: a test double returns `RecordFile` values from memory, so the
    rules can be exercised without a repository on disk.
    """

    def record_files(self, family: str) -> list[RecordFile]:
        """Return the record files in a family such as `"adrs"`.

        Parameters
        ----------
        family : str
            Record family directory name, such as `"adrs"` or `"rfcs"`.

        Returns
        -------
        list[RecordFile]
            The family's records, each with its name, relative path, and text.
        """
        ...

    def markdown_documents(self) -> list[MarkdownDocument]:
        """Return every Markdown document whose links must be inspected.

        Returns
        -------
        list[MarkdownDocument]
            Markdown documents, each with its relative path and text.
        """
        ...

    def link_facts(
        self,
        source: str,
        destination_path: str,
    ) -> LinkFacts | None:
        """Describe a local link target, or return `None` when it is malformed.

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
            Whether the target is inside the repository and exists, or `None`
            when the destination cannot be interpreted as a path.

        Example: from `docs/contents.md`, the destination
        `adrs/adr-0001-example.md` resolves to
        `docs/adrs/adr-0001-example.md`.
        """
        ...