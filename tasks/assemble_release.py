"""Put the pending news into the changelog."""

from __future__ import annotations

import argparse
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence


def main(argv: Sequence[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Assemble the release history")
    parser.add_argument("version")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)
    assemble(args.root, args.version, datetime.now(UTC).date().isoformat())


def assemble(root: Path, version: str, date: str) -> None:
    fragments = sorted((root / "changelog.d").glob("*.md"))
    changelog = root / "CHANGELOG.md"
    heading = "# Changelog\n"
    history = changelog.read_text(encoding="utf-8")
    if not history.startswith(heading):
        msg = f"{changelog} must start with {heading!r}"
        raise ValueError(msg)
    entry = "\n".join(release_section(fragments, f"{version} ({date})", f"release-{version.replace('.', '-')}"))
    changelog.write_text(f"{heading}\n{entry}{history.removeprefix(heading)}", encoding="utf-8")
    for fragment in fragments:
        fragment.unlink()


def release_section(fragments: Sequence[Path], title: str, anchor: str) -> list[str]:
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
    sections: dict[str, list[str]] = {}
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
        entry = " ".join(line.strip() for line in content.splitlines())
        sections.setdefault(names[category], []).append(f"- {entry}")
    release = [f"## {title}", "", f'<a name="{anchor}"></a>', ""]
    for category in dict.fromkeys(names.values()):
        if items := sections.get(category):
            release.extend((
                f'<a name="{anchor}-{category.lower().replace(" ", "-")}"></a>',
                "",
                f"### {category}",
                "",
                *items,
                "",
            ))
    return release


if __name__ == "__main__":
    main()


__all__ = [
    "assemble",
    "main",
    "release_section",
]
