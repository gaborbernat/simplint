"""Reject absolute links to this project's rule pages in changelog sources."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from typing import Final

_RULE_PAGE: Final[re.Pattern[str]] = re.compile(r"https?://simplint\.readthedocs\.io/(?:en/[^/]+/)?_generated/rules/")


def main(paths: list[str]) -> int:
    failed = False
    for raw_path in paths:
        path = Path(raw_path)
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if _RULE_PAGE.search(line):
                print(
                    f"{path}:{line_number}: use a bare backticked long rule name; changelog links are generated",
                    file=sys.stderr,
                )
                failed = True
    return int(failed)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))


__all__ = [
    "main",
]
