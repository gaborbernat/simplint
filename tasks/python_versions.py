# /// script
# requires-python = ">=3.12"
# dependencies = ["trove-classifiers>=2026.6.1.19"]
# ///
"""Update the package's Python requirement and PyPI classifiers."""

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


def newest_candidate() -> int:
    return max(with_a_candidate() & with_a_classifier())


def with_a_candidate() -> set[int]:
    released = fetch("https://www.python.org/api/v2/downloads/release/?is_published=true")
    release_candidate = re.compile(
        rf"""
    ^Python\ {_MAJOR}\.(?P<minor>\d+)\.\d+rc\d+$  # Published release-candidate names.
    """,
        re.VERBOSE,
    )
    return {int(found["minor"]) for release in released if (found := release_candidate.match(release["name"]))}


def with_a_classifier() -> set[int]:
    prefix = f"Programming Language :: Python :: {_MAJOR}."
    return {
        int(minor) for name in classifiers if name.startswith(prefix) and (minor := name.removeprefix(prefix)).isdigit()
    }


def oldest_supported() -> int:
    today = datetime.now(tz=UTC).date().isoformat()
    cycles = fetch("https://endoflife.date/api/python.json")
    live = (row for row in cycles if isinstance(row["eol"], str) and row["eol"] > today)
    return min(int(row["cycle"].split(".")[1]) for row in live)


def fetch(url: str) -> list[dict[str, str]]:
    with urllib.request.urlopen(url, timeout=30) as response:
        decoded: list[dict[str, str]] = json.load(response)
    return decoded


_CLASSIFIER_RUN_PATTERN: Final[re.Pattern[str]] = re.compile(
    r"""(?:\ \ "Programming\ Language\ ::\ Python\ ::\ \d+\.\d+",\n)+""",  # The per-release classifier run.
    re.VERBOSE,
)
_REQUIRES_PYTHON_PATTERN: Final[re.Pattern[str]] = re.compile(
    r'''requires-python\ =\ ">=\d+\.\d+"''',  # The package's own floor.
    re.VERBOSE,
)


def written_places(newest: int, oldest: int) -> list[Place]:
    # The package requirement sets the lower bound. The classifiers list each supported minor version.
    run = "".join(f'  "Programming Language :: Python :: {_MAJOR}.{minor}",\n' for minor in range(oldest, newest + 1))
    return [
        Place(
            path=_ROOT / "pyproject.toml",
            pattern=_REQUIRES_PYTHON_PATTERN,
            spelling=f'requires-python = ">={_MAJOR}.{oldest}"',
        ),
        Place(path=_ROOT / "pyproject.toml", pattern=_CLASSIFIER_RUN_PATTERN, spelling=run),
    ]


@dataclass(frozen=True, slots=True, kw_only=True)
class Place:
    path: Path
    pattern: re.Pattern[str]
    spelling: str

    def said(self) -> str:
        found = self.pattern.search(self.path.read_text(encoding="utf-8"))
        if found is None:
            print(f"{self.path.relative_to(_ROOT)}: nothing matches {self.pattern.pattern}")
            sys.exit(1)
        return found.group(0)

    def write(self, said: str) -> None:
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
