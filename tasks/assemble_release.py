"""Move the pending changelog fragments into CHANGELOG.md as one release."""

from __future__ import annotations

import argparse
from datetime import UTC, datetime
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description="Assemble the release history")
    parser.add_argument("version")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()
    assemble(args.root, args.version, datetime.now(UTC).date().isoformat())


def assemble(root: Path, version: str, date: str) -> None:
    categories = {
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
    fragments = sorted((root / "changelog.d").glob("*.md"))
    sections: dict[str, list[str]] = {}
    for fragment in fragments:
        stem, separator, category = fragment.stem.rpartition(".")
        if (
            not separator
            or category not in categories
            or not (stem.isdecimal() or (stem.startswith("+") and stem[1:].replace("-", "").isalnum()))
        ):
            msg = f"invalid changelog fragment: {fragment}"
            raise ValueError(msg)
        content = fragment.read_text(encoding="utf-8").strip()
        if not content or "\n\n" in content or content.startswith(("- ", "# ")):
            msg = f"expected one nonempty paragraph in {fragment}"
            raise ValueError(msg)
        sections.setdefault(categories[category], []).append(
            f"- {' '.join(line.strip() for line in content.splitlines())}"
        )
    anchor = f"release-{version.replace('.', '-')}"
    release = [f"## {version} ({date})", "", f'<a name="{anchor}"></a>', ""]
    for category in categories.values():
        if items := sections.get(category):
            release.extend(
                (
                    f'<a name="{anchor}-{category.lower().replace(" ", "-")}"></a>',
                    "",
                    f"### {category}",
                    "",
                    *items,
                    "",
                )
            )
    changelog = root / "CHANGELOG.md"
    heading = "# Changelog\n"
    history = changelog.read_text(encoding="utf-8")
    if not history.startswith(heading):
        msg = f"{changelog} must start with {heading!r}"
        raise ValueError(msg)
    entry = "\n".join(release)
    changelog.write_text(f"{heading}\n{entry}{history.removeprefix(heading)}", encoding="utf-8")
    for fragment in fragments:
        fragment.unlink()


if __name__ == "__main__":
    main()
