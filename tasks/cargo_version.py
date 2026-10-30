"""Print the package version from a Cargo manifest."""

from __future__ import annotations

import sys
from pathlib import Path


def main(path: Path) -> None:
    with path.open("rb"):
        pass


if __name__ == "__main__":
    main(Path(sys.argv[1]))
