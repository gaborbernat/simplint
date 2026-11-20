"""Put the pending news into the changelog."""

from __future__ import annotations

import argparse
import re
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING, Final

if TYPE_CHECKING:
    from collections.abc import Sequence

#: The changelog's title, which the release history and the docs page both start from.
TITLE: Final[str] = "# Changelog"
#: generate_changelog reads these anchors to preserve links to releases and categories.
ANCHOR: Final[re.Pattern[str]] = re.compile(r'<a name="(?P<name>[^"]+)"></a>')
RULE_SOURCE_LINK: Final[re.Pattern[str]] = re.compile(
    r"""
    \[`(?P<name>[a-z0-9-]+)`\]  # long rule name
    \(                            # Markdown link target
    (?P<path>
        (?:\.\./                  # a fragment sits one directory below the root
        |https://github\.com/gaborbernat/simplint/blob/[^/)]+/  # the changelog links the release's file
        )?
        (?P<source>               # path from the root
            src/rules/             # rule source directory
            [a-z0-9_/]+\.md       # rule source file
        )
    )
    \)                            # end of target
    """,
    re.VERBOSE,
)
# A link a fragment writes from its own directory to a file of the project.
_FRAGMENT_LINK: Final[re.Pattern[str]] = re.compile(r"\]\(\.\./(?P<path>[^)]+)\)")


def rule_source_name(match: re.Match[str]) -> str:
    if Path(match["path"]).stem != match["name"].replace("-", "_"):
        msg = f"rule source link for {match['name']} points to {match['path']}"
        raise ValueError(msg)
    return match["name"]


def main(argv: Sequence[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Assemble the release history")
    parser.add_argument("version")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)
    assemble(args.root, args.version, datetime.now(UTC).date().isoformat())


def assemble(root: Path, version: str, date: str) -> None:
    changelog = root / "CHANGELOG.md"
    history = changelog.read_text(encoding="utf-8")
    heading = f"{TITLE}\n"
    if not history.startswith(heading):
        msg = f"{changelog} must start with {heading!r}"
        raise ValueError(msg)
    fragments = sorted((root / "changelog.d").glob("*.md"))
    entry = "\n".join(release_section(fragments, f"{version} ({date})", f"release-{version.replace('.', '-')}"))

    for match in RULE_SOURCE_LINK.finditer(entry):
        rule_source_name(match)
    # The release notes also appear on GitHub and PyPI. Pin file links to the tag so they work in both places.
    blob = f"https://github.com/gaborbernat/simplint/blob/{version}"
    entry = _FRAGMENT_LINK.sub(lambda match: f"]({blob}/{match['path']})", entry)
    changelog.write_text(f"{heading}\n{entry}{history.removeprefix(heading)}", encoding="utf-8")
    for fragment in fragments:
        fragment.unlink()


def release_section(fragments: Sequence[Path], title: str, anchor: str) -> list[str]:
    sections: dict[str, list[str]] = {}
    names = {
        "feature": "Added",
        "change": "Changes",
        "bugfix": "Fixed",
        "doc": "Documentation",
        "breaking": "Breaking changes",
        "security": "Security",
        "performance": "Performance",
        "deprecation": "Deprecated",
        "removal": "Removed",
        "packaging": "Packaging",
    }
    for fragment in fragments:
        stem, separator, category = fragment.stem.rpartition(".")
        if (
            not separator
            or not stem
            or category not in names
            or not (stem.isdecimal() or (stem.startswith("+") and stem[1:].replace("-", "").isalnum()))
        ):
            msg = f"invalid changelog fragment: {fragment}"
            raise ValueError(msg)
        content = fragment.read_text(encoding="utf-8").strip()
        if not content or "\n\n" in content or content.startswith(("- ", "# ")):
            msg = f"expected one nonempty paragraph in {fragment}"
            raise ValueError(msg)
        sections.setdefault(names[category], []).append(f"- {' '.join(line.strip() for line in content.splitlines())}")
    release = [f"## {title}", "", _anchor_line(anchor), ""]
    for category in dict.fromkeys(names.values()):
        if items := sections.get(category):
            release.extend((
                _anchor_line(f"{anchor}-{category.lower().replace(' ', '-')}"),
                "",
                f"### {category}",
                "",
                *items,
                "",
            ))
    return release


def _anchor_line(name: str) -> str:
    return f'<a name="{name}"></a>'


if __name__ == "__main__":
    main()


__all__ = [
    "ANCHOR",
    "RULE_SOURCE_LINK",
    "TITLE",
    "assemble",
    "main",
    "release_section",
    "rule_source_name",
]
