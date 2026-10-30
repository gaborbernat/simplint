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
    newest, oldest = newest_candidate(), oldest_supported()
    places = written_places(newest, oldest)
    drifted = [(place, said) for place in places if (said := place.said()) != place.spelling]
    if not drifted:
        print(f"the defaults name {_MAJOR}.{oldest} through {_MAJOR}.{newest}")
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


# The oldest release still carrying fixes is the target version simplint assumes when a project names none.
def oldest_supported() -> int:
    today = datetime.now(tz=UTC).date().isoformat()
    cycles = fetch("https://endoflife.date/api/python.json")
    live = (row for row in cycles if isinstance(row["eol"], str) and row["eol"] > today)
    return min(int(row["cycle"].split(".")[1]) for row in live)


def fetch(url: str) -> list[dict[str, str]]:
    with urllib.request.urlopen(url, timeout=30) as response:
        decoded: list[dict[str, str]] = json.load(response)
    return decoded


def written_places(newest: int, oldest: int) -> list[Place]:
    classifier_run = re.compile(
        r"""(?:\ \ "Programming\ Language\ ::\ Python\ ::\ \d+\.\d+",\n)+""",  # The per-release classifier run.
        re.VERBOSE,
    )
    run = "".join(f'  "Programming Language :: Python :: {_MAJOR}.{minor}",\n' for minor in range(oldest, newest + 1))
    default_target = re.compile(
        r"""target_version:\ PythonVersion::PY\d+,""",  # Fallback target version in the settings.
        re.VERBOSE,
    )
    doc_target = re.compile(
        r"""then\ to\ Python\ \d+\.\d+""",  # The documented fallback.
        re.VERBOSE,
    )
    requires_python = re.compile(
        r'''requires-python\ =\ ">=\d+\.\d+"''',  # The package's own floor.
        re.VERBOSE,
    )
    return [
        Place(
            path=_ROOT / "src/settings.rs",
            pattern=default_target,
            spelling=f"target_version: PythonVersion::PY{_MAJOR}{oldest},",
        ),
        Place(path=_ROOT / "docs/configuration.rst", pattern=doc_target, spelling=f"then to Python {_MAJOR}.{oldest}"),
        Place(
            path=_ROOT / "pyproject.toml", pattern=requires_python, spelling=f'requires-python = ">={_MAJOR}.{oldest}"'
        ),
        Place(path=_ROOT / "pyproject.toml", pattern=classifier_run, spelling=run),
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
