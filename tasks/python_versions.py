# /// script
# requires-python = ">=3.12"
# dependencies = ["trove-classifiers>=2026.6.1.19"]
# ///
"""Keep the Python releases simplint's defaults stand for in step with what Python and PyPI publish."""

from __future__ import annotations

import json
import re
import sys
import urllib.request
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Final

from trove_classifiers import classifiers

_ROOT: Final[Path] = Path(__file__).resolve().parents[1]
_MAJOR: Final[int] = 3


def main(*, check: bool) -> None:
    """
    Report the places that name outdated Python releases and rewrite them.

    :param check: exit with status 1 on drift instead of rewriting the places
    """
    newest, oldest = newest_candidate(), oldest_supported()
    places = written_places(newest, oldest)
    drifted = [(place, said) for place in places if (said := place.said()) != place.spelling]
    if not drifted:
        print(f"the package names {_MAJOR}.{oldest} through {_MAJOR}.{newest}")
        return
    for place, said in drifted:
        print(f"{place.path.relative_to(_ROOT)}: {said!r} -> {place.spelling!r}")
    if check:
        sys.exit(1)
    for place, said in drifted:
        place.write(said)


# A release still in alpha or beta is one whose own behavior is still moving, and a candidate PyPI publishes no
# classifier for is one no project can name, so the newest release needs both.
def newest_candidate() -> int:
    """
    Find the newest Python minor release that has a release candidate and a trove classifier.

    :returns: the minor version number
    """
    return max(with_a_candidate() & with_a_classifier())


def with_a_candidate() -> set[int]:
    """
    List the Python minor releases that python.org has published a release candidate for.

    :returns: the minor version numbers
    """
    released = fetch("https://www.python.org/api/v2/downloads/release/?is_published=true")
    release_candidate = re.compile(
        rf"""
    ^Python\ {_MAJOR}\.(?P<minor>\d+)\.\d+rc\d+$  # Published release-candidate names.
    """,
        re.VERBOSE,
    )
    return {int(found["minor"]) for release in released if (found := release_candidate.match(release["name"]))}


def with_a_classifier() -> set[int]:
    """
    List the Python minor releases that have a trove classifier.

    :returns: the minor version numbers
    """
    prefix = f"Programming Language :: Python :: {_MAJOR}."
    return {
        int(minor) for name in classifiers if name.startswith(prefix) and (minor := name.removeprefix(prefix)).isdigit()
    }


# The oldest release still carrying fixes is the oldest the package installs on; the target version a project that names
# none gets is Ruff's default, which Ruff maintains.
def oldest_supported() -> int:
    """
    Find the oldest Python minor release that has not reached end of life.

    :returns: the minor version number
    """
    today = datetime.now(tz=UTC).date().isoformat()
    cycles = fetch("https://endoflife.date/api/python.json")
    live = (row for row in cycles if isinstance(row["eol"], str) and row["eol"] > today)
    return min(int(row["cycle"].split(".")[1]) for row in live)


def fetch(url: str) -> list[dict[str, str]]:
    """
    Download and decode a JSON list.

    :param url: the address to fetch
    :returns: the decoded rows
    """
    with urllib.request.urlopen(url, timeout=30) as response:
        decoded: list[dict[str, str]] = json.load(response)
    return decoded


def written_places(newest: int, oldest: int) -> list[Place]:
    """
    Describe each place in the project that names the supported Python releases.

    :param newest: the newest supported minor release
    :param oldest: the oldest supported minor release
    :returns: the places with their expected spelling
    """
    classifier_run = re.compile(
        r"""(?:\ \ "Programming\ Language\ ::\ Python\ ::\ \d+\.\d+",\n)+""",  # The per-release classifier run.
        re.VERBOSE,
    )
    run = "".join(f'  "Programming Language :: Python :: {_MAJOR}.{minor}",\n' for minor in range(oldest, newest + 1))
    requires_python = re.compile(
        r'''requires-python\ =\ ">=\d+\.\d+"''',  # The package's own floor.
        re.VERBOSE,
    )
    return [
        Place(
            path=_ROOT / "pyproject.toml", pattern=requires_python, spelling=f'requires-python = ">={_MAJOR}.{oldest}"'
        ),
        Place(path=_ROOT / "pyproject.toml", pattern=classifier_run, spelling=run),
    ]


@dataclass(frozen=True, slots=True, kw_only=True)
class Place:
    """A spot in a file that names a Python release, with the text it should hold."""

    path: Path
    pattern: re.Pattern[str]
    spelling: str

    def said(self) -> str:
        """
        Return the text the file holds at this place.

        :returns: the matched text
        :raises SystemExit: if the pattern matches nothing in the file
        """
        found = self.pattern.search(self.path.read_text(encoding="utf-8"))
        if found is None:
            print(f"{self.path.relative_to(_ROOT)}: nothing matches {self.pattern.pattern}")
            sys.exit(1)
        return found.group(0)

    def write(self, said: str) -> None:
        """
        Replace the text in the file with the expected spelling.

        :param said: the text the file holds now
        """
        self.path.write_text(self.path.read_text(encoding="utf-8").replace(said, self.spelling), encoding="utf-8")


if __name__ == "__main__":
    main(check="--check" in sys.argv[1:])


__all__ = [
    "Place",
    "fetch",
    "main",
    "newest_candidate",
    "oldest_supported",
    "with_a_candidate",
    "with_a_classifier",
    "written_places",
]
