"""Render the Markdown release history, pending news included, as an RST documentation page."""

from __future__ import annotations

import re
from pathlib import Path

import pypandoc

from docs.rules_data import rule_entries
from tasks.assemble_release import ANCHOR, RULE_SOURCE_LINK, TITLE, release_section, rule_source_name


def main(root: Path, rules: dict[str, str]) -> None:
    # Write ``docs/changelog.rst`` from the changelog and the pending fragments.
    #
    # :param root: the project root
    # :param rules: rule names keyed by code, used to link rule mentions
    # :raises ValueError: if an anchor has no heading to attach to
    history = (root / "CHANGELOG.md").read_text(encoding="utf-8").removeprefix(f"{TITLE}\n").strip()
    pending = sorted((root / "changelog.d").glob("*.md"))
    draft = "\n".join(release_section(pending, "Draft", "release-draft")) if pending else ""
    # Released history keeps its codes as plain text: a code there meant the rule it had before the renumbering.
    parts = (link_rules(draft, rules, codes=True), link_rules(history, rules, codes=False))
    lines = "\n\n".join(part for part in (TITLE, *parts) if part).splitlines()
    # Each anchor line becomes an attribute of the heading it names, and the anchor lines themselves go.
    anchors = _anchors(lines)
    marked = "\n".join(
        f"{line} {{#{anchors[index]}}}" if index in anchors else line
        for index, line in enumerate(lines)
        if not ANCHOR.fullmatch(line)
    )
    converted = pypandoc.convert_text(marked, "rst", format="markdown+header_attributes", extra_args=["--columns=120"])
    (root / "docs" / "changelog.rst").write_text(converted, encoding="utf-8")


# The anchor names keyed by the index of the heading line each attaches to.
def _anchors(lines: list[str]) -> dict[int, str]:
    anchors: dict[int, str] = {}
    for index, line in enumerate(lines):
        if not (match := ANCHOR.fullmatch(line)):
            continue
        heading = _anchor_heading(lines, index)
        if heading is None or heading in anchors:
            msg = f"cannot attach changelog anchor {line} to a heading"
            raise ValueError(msg)
        anchors[heading] = match["name"]
    return anchors


# A category anchor leads its `###` heading; a release anchor trails the version heading above it, even when the release
# lists nothing and the next version's heading follows.
def _anchor_heading(lines: list[str], index: int) -> int | None:
    following = next((position for position in range(index + 1, len(lines)) if lines[position].strip()), None)
    if following is not None and lines[following].startswith("### "):
        return following
    return next((position for position in range(index - 1, -1, -1) if lines[position].startswith("#")), None)


def link_rules(markdown: str, rules: dict[str, str], *, codes: bool) -> str:
    # Link each backticked rule name to its page, and each code too when `codes` is set.
    by_name = {name: code for code, name in rules.items()}

    def source_link(match: re.Match[str]) -> str:
        name = rule_source_name(match)
        if name not in by_name:
            msg = f"unknown rule source link: {name}"
            raise ValueError(msg)
        return f"`{name}`"

    markdown = RULE_SOURCE_LINK.sub(source_link, markdown)

    # A raw `:doc:` role lets Sphinx resolve the page and fail the build when it is missing; a Markdown link would turn
    # into an external URL that neither Sphinx nor linkcheck can follow.
    def link(match: re.Match[str]) -> str:
        code = match.groupdict().get("code") or by_name[match["name"]]
        return f"`` :doc:`{rules[code]} <rules/{code}>` ``{{=rst}}"

    named = f"`(?P<name>{'|'.join(re.escape(name) for name in sorted(by_name, key=len, reverse=True)) or '(?!)'})`"
    if not codes:
        return re.sub(named, link, markdown)
    # The link already names the rule, so a code right after it would link the same rule twice; the code goes.
    code_pattern = "|".join(rules) or "(?!)"
    return re.sub(rf"\b(?P<code>{code_pattern})\b|{named}(?: \((?:{code_pattern})\))?", link, markdown)


def rule_names(root: Path) -> dict[str, str]:
    # Read the rule names from the sources the binary's rule list is built from.
    #
    # :param root: the project root
    # :returns: rule names keyed by code
    return {rule["code"]: rule["name"] for rule in rule_entries(root)}


if __name__ == "__main__":
    _ROOT = Path(__file__).resolve().parents[1]
    main(_ROOT, rule_names(_ROOT))


__all__ = [
    "link_rules",
    "main",
    "rule_names",
]
