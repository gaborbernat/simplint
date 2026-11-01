"""Render the Markdown release history, pending news included, as an RST documentation page."""

from __future__ import annotations

import json
import re
import shutil
import subprocess
from pathlib import Path

import pypandoc

from tasks.assemble_release import release_section


def main(root: Path, rules: dict[str, str]) -> None:
    """
    Write ``docs/changelog.rst`` from the changelog and the pending fragments.

    :param root: the project root
    :param rules: rule names keyed by code, used to link rule mentions
    :raises ValueError: if an anchor has no heading to attach to
    """
    history = (root / "CHANGELOG.md").read_text(encoding="utf-8").removeprefix("# Changelog\n").strip()
    pending = sorted((root / "changelog.d").glob("*.md"))
    draft = "\n".join(release_section(pending, "Draft", "release-draft")) if pending else ""
    source = link_rules("\n\n".join(part for part in ("# Changelog", draft, history) if part), rules)
    lines = source.splitlines()
    anchors: dict[int, str] = {}
    for index, line in enumerate(lines):
        if not line.startswith('<a name="'):
            continue
        # A category anchor leads its heading; a release anchor trails the version heading above it.
        following = next((position for position in range(index + 1, len(lines)) if lines[position].strip()), None)
        heading = (
            following
            if following is not None and lines[following].startswith("#")
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


def link_rules(markdown: str, rules: dict[str, str]) -> str:
    """Link each rule, named by its code or its backticked name, to its page under its name."""
    codes = {name: code for code, name in rules.items()}

    # A raw `:doc:` role lets Sphinx resolve the page and fail the build when it is missing; a Markdown link would turn
    # into an external URL that neither Sphinx nor linkcheck can follow.
    def link(match: re.Match[str]) -> str:
        code = match["code"] or codes[match["name"]]
        return f"`` :doc:`{rules[code]} <_generated/rules/{code}>` ``{{=rst}}"

    names = "|".join(re.escape(name) for name in sorted(codes, key=len, reverse=True)) or "(?!)"
    return re.sub(rf"\b(?P<code>{'|'.join(rules) or '(?!)'})\b|`(?P<name>{names})`", link, markdown)


def rule_names() -> dict[str, str]:
    """
    Read the rule names from the installed ``simplint`` binary.

    :returns: rule names keyed by code, empty when the binary is not on the path
    :raises subprocess.CalledProcessError: if the binary fails to list its rules
    """
    # the docs environment installs the binary, whose rule list maps each code to its name
    if (binary := shutil.which("simplint")) is None:
        return {}
    listing = subprocess.run([binary, "rules", "--output-format", "json"], check=True, capture_output=True, text=True)
    return {rule["code"]: rule["name"] for rule in json.loads(listing.stdout)}


if __name__ == "__main__":
    main(Path(__file__).resolve().parents[1], rule_names())


__all__ = [
    "link_rules",
    "main",
    "rule_names",
]
