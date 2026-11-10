"""Reject absolute links to this project's rule pages in changelog sources."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Final

from tasks.assemble_release import RULE_SOURCE_LINK, rule_source_name

_RULE_PAGE: Final[re.Pattern[str]] = re.compile(r"https?://simplint\.readthedocs\.io/(?:en/[^/]+/)?rules/")
# Only a link of the exact RULE_SOURCE_LINK form becomes a rule page link; any other link into the rule sources stays a
# relative path, which the published changelog resolves outside the documentation.
_ANY_SOURCE_LINK: Final[re.Pattern[str]] = re.compile(
    r"""
    \[[^\]]*\]  # any link text
    \(          # Markdown link target
    (?:\.\./)?  # fragment files sit one directory below the root
    src/rules/  # rule source directory
    [^)]*\)     # rest of the target
    """,
    re.VERBOSE,
)


def main(paths: list[str]) -> int:
    failed = False
    known_names: dict[Path, set[str]] = {}
    for raw_path in paths:
        path = Path(raw_path)
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if _RULE_PAGE.search(line):
                print(
                    f"{path}:{line_number}: link to the rule source with a relative Markdown path",
                    file=sys.stderr,
                )
                failed = True
            for link in _ANY_SOURCE_LINK.finditer(line):
                if not RULE_SOURCE_LINK.fullmatch(link[0]):
                    print(
                        f"{path}:{line_number}: write the rule source link as [`name`](path): {link[0]}",
                        file=sys.stderr,
                    )
                    failed = True
            for match in RULE_SOURCE_LINK.finditer(line):
                try:
                    _validate_source_link(match, path, known_names)
                except ValueError as error:
                    print(f"{path}:{line_number}: {error}", file=sys.stderr)
                    failed = True
    return int(failed)


def _validate_source_link(match: re.Match[str], path: Path, known_names: dict[Path, set[str]]) -> None:
    name = rule_source_name(match)
    if not (path.parent / match["path"]).is_file():
        msg = f"missing rule source for {match[0]}"
        raise ValueError(msg)
    root = path.parent if path.name == "CHANGELOG.md" else path.parent.parent
    if root not in known_names:
        rules = json.loads((root / "docs" / "_data" / "rules.json").read_text(encoding="utf-8"))
        known_names[root] = {rule["name"] for rule in rules}
    if name not in known_names[root]:
        msg = f"unknown rule source link: {name}"
        raise ValueError(msg)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))


__all__ = [
    "main",
]
