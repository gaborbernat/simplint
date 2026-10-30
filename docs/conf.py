"""Sphinx configuration for the simplint documentation."""

from __future__ import annotations

import json
import shutil
import subprocess
from datetime import UTC, datetime
from importlib.metadata import version as metadata_version
from pathlib import Path
from typing import Final, TypedDict

project: Final[str] = "simplint"
release: Final[str] = metadata_version("simplint")
version: Final[str] = ".".join(release.split(".")[:2])
project_copyright: Final[str] = f"2026-{datetime.now(tz=UTC).year}, Bernát Gábor"

extensions: Final[list[str]] = ["myst_parser", "sphinx_copybutton"]
myst_enable_extensions: Final[list[str]] = ["colon_fence", "deflist"]
myst_heading_anchors: Final[int] = 3
exclude_patterns: Final[list[str]] = ["_build"]

html_theme: Final[str] = "furo"
html_title: Final[str] = "simplint"
html_theme_options: Final[dict[str, str]] = {
    "source_repository": "https://github.com/gaborbernat/simplint",
    "source_branch": "main",
    "source_directory": "docs/",
}
pygments_style, pygments_dark_style = "sphinx", "monokai"
linkcheck_anchors_ignore_for_url: Final[list[str]] = [r"https://github\.com/.*"]


class _RuleEntry(TypedDict):
    code: str
    name: str
    summary: str
    fix: str
    explanation: str


def _write_rule_pages(docs: Path) -> None:
    # The binary embeds each rule's explanation, so `simplint rule` and these pages cannot drift apart.
    if (binary := shutil.which("simplint")) is None:
        msg = "simplint is not on PATH; build the docs through `tox run -e docs`"
        raise FileNotFoundError(msg)
    output = subprocess.run([binary, "rules", "--output-format", "json"], check=True, capture_output=True, text=True)
    rules: list[_RuleEntry] = json.loads(output.stdout)
    pages = docs / "rules"
    shutil.rmtree(pages, ignore_errors=True)
    pages.mkdir()
    fix_notes = {
        "always": "Fix is always available.",
        "sometimes": "Fix is sometimes available.",
        "none": "No fix is available.",
    }
    groups = {
        "SMP0": "Comments, docstrings and structure",
        "SMP1": "Modules, typing and exports",
        "SMP2": "Tests",
    }
    index = [
        "# Rules",
        "",
        "🛠️ marks a rule whose fix is always available and 🔧 one whose fix covers some cases. `--fix` applies safe",
        "fixes and `--unsafe-fixes` the rest; each rule page says which applies.",
        "",
    ]
    for prefix, title in groups.items():
        index.extend([f"## {title}", "", "| Code | Name | Checks | Fix |", "| --- | --- | --- | --- |"])
        for rule in (entry for entry in rules if entry["code"].startswith(prefix)):
            fix = {"always": "🛠️", "sometimes": "🔧", "none": ""}[rule["fix"]]
            index.append(f"| [{rule['code']}](rules/{rule['code']}.md) | {rule['name']} | {rule['summary']} | {fix} |")
        index.append("")
    index.extend([":::{toctree}", ":hidden:", "", *(f"rules/{rule['code']}" for rule in rules), ":::", ""])
    (docs / "rules.md").write_text("\n".join(index), encoding="utf-8")
    for rule in rules:
        page = f"# {rule['name']} ({rule['code']})\n\n{fix_notes[rule['fix']]}\n\n{rule['explanation']}"
        (pages / f"{rule['code']}.md").write_text(page, encoding="utf-8")


_write_rule_pages(Path(__file__).parent)
