"""Render the Markdown release history, pending news included, as an RST documentation page."""

from __future__ import annotations

from pathlib import Path

import pypandoc

from tasks.assemble_release import release_section


def main(root: Path) -> None:
    history = (root / "CHANGELOG.md").read_text(encoding="utf-8").removeprefix("# Changelog\n").strip()
    pending = sorted((root / "changelog.d").glob("*.md"))
    draft = "\n".join(release_section(pending, "Draft", "release-draft")) if pending else ""
    source = "\n\n".join(part for part in ("# Changelog", draft, history) if part)
    lines = source.splitlines()
    anchors: dict[int, str] = {}
    for index, line in enumerate(lines):
        if not line.startswith('<a name="'):
            continue
        # A category anchor leads its heading; a release anchor trails the version heading above it.
        following = next((position for position in range(index + 1, len(lines)) if lines[position].strip()), None)
        heading = (
            following
            if following is not None and lines[following].startswith("#")
            else next((position for position in range(index - 1, -1, -1) if lines[position].startswith("#")), None)
        )
        if heading is None or heading in anchors:
            msg = f"cannot attach changelog anchor {line} to a heading"
            raise ValueError(msg)
        anchors[heading] = line.removeprefix('<a name="').removesuffix('"></a>')
    marked = "\n".join(
        f"{line} {{#{anchors[index]}}}" if index in anchors else line
        for index, line in enumerate(lines)
        if not line.startswith('<a name="')
    )
    converted = pypandoc.convert_text(marked, "rst", format="markdown+header_attributes", extra_args=["--columns=120"])
    (root / "docs" / "changelog.rst").write_text(converted, encoding="utf-8")


if __name__ == "__main__":
    main(Path(__file__).resolve().parents[1])


__all__ = [
    "main",
]
