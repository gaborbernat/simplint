"""Render the Markdown release history, pending news included, as an RST documentation page."""

from __future__ import annotations

import json
import re
from pathlib import Path

import pypandoc

from tasks.assemble_release import release_section


def main(root: Path, rules: dict[str, str]) -> None:
    # Write ``docs/changelog.rst`` from the changelog and the pending fragments.
    #
    # :param root: the project root
    # :param rules: rule names keyed by code, used to link rule mentions
    # :raises ValueError: if an anchor has no heading to attach to
    history = (root / "CHANGELOG.md").read_text(encoding="utf-8").removeprefix("# Changelog\n").strip()
    pending = sorted((root / "changelog.d").glob("*.md"))
    draft = "\n".join(release_section(pending, "Draft", "release-draft")) if pending else ""
    # Released history keeps its codes as plain text: a code there meant the rule it had before the renumbering.
    parts = (link_rules(draft, rules, codes=True), link_rules(history, rules, codes=False))
    lines = "\n\n".join(part for part in ("# Changelog", *parts) if part).splitlines()
    anchors: dict[int, str] = {}
    for index, line in enumerate(lines):
        if not line.startswith('<a name="'):
            continue
        # A category anchor leads its `###` heading; a release anchor trails the version heading above it, even when the
        # release lists nothing and the next version's heading follows.
        following = next((position for position in range(index + 1, len(lines)) if lines[position].strip()), None)
        heading = (
            following
            if following is not None and lines[following].startswith("### ")
            else next((position for position in range(index - 1, -1, -1) if lines[position].startswith("#")), None)
        )
        if heading is None or heading in anchors:
            msg = f"cannot attach changelog anchor {line} to a heading"
            raise ValueError(msg)
        anchors[heading] = line.removeprefix('<a name="').removesuffix('"></a>')
    marked = "\n".join(
        f"{line} {{#{anchors[index]}}}" if index in anchors else line
        for index, line in enumerate(lines)
        if not line.startswith('<a name="')
    )
    converted = pypandoc.convert_text(marked, "rst", format="markdown+header_attributes", extra_args=["--columns=120"])
    (root / "docs" / "changelog.rst").write_text(converted, encoding="utf-8")


def link_rules(markdown: str, rules: dict[str, str], *, codes: bool) -> str:
    # Link each backticked rule name to its page, and each code too when `codes` is set.
    by_name = {name: code for code, name in rules.items()}

    # A raw `:doc:` role lets Sphinx resolve the page and fail the build when it is missing; a Markdown link would turn
    # into an external URL that neither Sphinx nor linkcheck can follow.
    def link(match: re.Match[str]) -> str:
        code = match.groupdict().get("code") or by_name[match["name"]]
        return f"`` :doc:`{rules[code]} <_generated/rules/{code}>` ``{{=rst}}"

    named = f"`(?P<name>{'|'.join(re.escape(name) for name in sorted(by_name, key=len, reverse=True)) or '(?!)'})`"
    if not codes:
        return re.sub(named, link, markdown)
    # The link already names the rule, so a code right after it would link the same rule twice; the code goes.
    code_pattern = "|".join(rules) or "(?!)"
    return re.sub(rf"\b(?P<code>{code_pattern})\b|{named}(?: \((?:{code_pattern})\))?", link, markdown)


def rule_names(root: Path) -> dict[str, str]:
    # Read the rule names from the docs' copy of the binary's rule list.
    #
    # :param root: the project root
    # :returns: rule names keyed by code
    rules = json.loads((root / "docs" / "_data" / "rules.json").read_text(encoding="utf-8"))
    return {rule["code"]: rule["name"] for rule in rules}


if __name__ == "__main__":
    _ROOT = Path(__file__).resolve().parents[1]
    main(_ROOT, rule_names(_ROOT))


__all__ = [
    "link_rules",
    "main",
    "rule_names",
]
