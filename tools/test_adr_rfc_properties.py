"""Property tests for ADR/RFC slug generation and Markdown extraction."""

from __future__ import annotations

import re

from hypothesis import given
from hypothesis import strategies as st

from adr_rfc_names import extract_markdown_destinations, slug_for_title


CANONICAL_SLUG = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")
DESTINATION = st.from_regex(r"[a-z0-9][a-z0-9/_.-]{0,30}\.md", fullmatch=True)
FENCE = st.sampled_from(("```", "````", "~~~", "~~~~"))


@given(title=st.text())
def test_slug_for_title_returns_canonical_kebab_case(title: str) -> None:
    slug = slug_for_title(title)

    assert slug == "" or CANONICAL_SLUG.fullmatch(slug), (
        "a generated slug must be canonical kebab-case"
    )


@given(title=st.text())
def test_slug_for_title_is_idempotent(title: str) -> None:
    slug = slug_for_title(title)

    assert slug_for_title(slug) == slug, "slug generation must be idempotent"


@given(fence=FENCE, destination=DESTINATION)
def test_fenced_links_are_hidden_while_visible_links_remain(
    fence: str, destination: str
) -> None:
    fenced = f"{fence}\n[link]({destination})\n{fence}\n"
    visible = f"[link]({destination})\n"

    assert extract_markdown_destinations(fenced) == [], (
        "links inside a fenced block must be hidden"
    )
    assert extract_markdown_destinations(fenced + visible) == [destination], (
        "only the visible link must remain"
    )


@given(destination=DESTINATION)
def test_html_record_destinations_are_extracted(destination: str) -> None:
    markdown = f'<a href="{destination}">record</a>\n'

    assert extract_markdown_destinations(markdown) == [destination], (
        "the HTML href must be extracted"
    )