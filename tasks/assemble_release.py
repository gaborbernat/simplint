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
#: The anchor line release_section writes above each release and category; generate_changelog reads them back.
ANCHOR: Final[re.Pattern[str]] = re.compile(r'<a name="(?P<name>[^"]+)"></a>')
RULE_SOURCE_LINK: Final[re.Pattern[str]] = re.compile(
    r"""
    \[`(?P<name>[a-z0-9-]+)`\]  # long rule name
    \(                            # Markdown link target
    (?P<path>                     # path from the changelog source
        (?:\.\./)?                # fragment files sit one directory below the root
        src/rules/                 # rule source directory
        [a-z0-9_/]+\.md           # rule source file
    )
    \)                            # end of target
    """,
    re.VERBOSE,
)


def rule_source_name(match: re.Match[str]) -> str:
    if Path(match["path"]).stem != match["name"].replace("-", "_"):
        msg = f"rule source link for {match['name']} points to {match['path']}"
        raise ValueError(msg)
    return match["name"]


def main(argv: Sequence[str] | None = None) -> None:
    # Assemble the pending fragments into a release dated today from the command line.
    #
    # :param argv: the arguments to parse, ``sys.argv`` when omitted
    parser = argparse.ArgumentParser(description="Assemble the release history")
    parser.add_argument("version")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)
    assemble(args.root, args.version, datetime.now(UTC).date().isoformat())


def assemble(root: Path, version: str, date: str) -> None:
    # Move the pending fragments into the changelog as a new release section and delete them.
    #
    # :param root: the project root holding ``CHANGELOG.md`` and ``changelog.d``
    # :param version: the version being released
    # :param date: the release date in ISO format
    # :raises ValueError: if the changelog lacks its heading or a fragment is invalid
    changelog = root / "CHANGELOG.md"
    history = changelog.read_text(encoding="utf-8")
    heading = f"{TITLE}\n"
    if not history.startswith(heading):
        msg = f"{changelog} must start with {heading!r}"
        raise ValueError(msg)
    fragments = sorted((root / "changelog.d").glob("*.md"))
    entry = "\n".join(release_section(fragments, f"{version} ({date})", f"release-{version.replace('.', '-')}"))

    def relocate(match: re.Match[str]) -> str:
        rule_source_name(match)
        return match[0].replace("(../src/rules/", "(src/rules/")

    entry = RULE_SOURCE_LINK.sub(relocate, entry)
    changelog.write_text(f"{heading}\n{entry}{history.removeprefix(heading)}", encoding="utf-8")
    for fragment in fragments:
        fragment.unlink()


def release_section(fragments: Sequence[Path], title: str, anchor: str) -> list[str]:
    # Render the fragments as Markdown lines of one release, grouped by category.
    #
    # :param fragments: the fragment files, named ``<issue>.<category>.md`` or ``+<slug>.<category>.md``
    # :param title: the release heading text
    # :param anchor: the HTML anchor of the release, also the prefix of each category anchor
    # :returns: the lines of the release section
    # :raises ValueError: if a fragment has an invalid name or is not one nonempty paragraph
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


# The line ANCHOR reads back.
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
