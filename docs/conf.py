"""Configure the documentation build."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from datetime import UTC, datetime
from importlib.metadata import version as metadata_version
from pathlib import Path
from typing import Final, TypedDict

import pypandoc

company, name = "gaborbernat", "simplint"
ver: Final[str] = metadata_version("simplint")
release, version = ver, ".".join(ver.split(".")[:2])
now: Final[datetime] = datetime.now(tz=UTC)
project_copyright: Final[str] = f"2026-{now.year}, {company}"
master_doc, source_suffix = "index", ".rst"

html_theme: Final[str] = "furo"
html_title, html_last_updated_fmt = name, now.isoformat()
pygments_style, pygments_dark_style = "sphinx", "monokai"

extensions: Final[list[str]] = [
    "sphinx_llm.txt",
    "sphinx.ext.extlinks",
    "sphinx.ext.intersphinx",
    "sphinx_inline_tabs",
    "sphinx_copybutton",
    "sphinx_design",
]

exclude_patterns: Final[list[str]] = ["_build", "changelog/*", "_draft.rst"]

extlinks: Final[dict[str, tuple[str, str]]] = {
    "pypi": ("https://pypi.org/project/%s", "%s"),
    "gh": ("https://github.com/%s", "%s"),
}
intersphinx_mapping: Final[dict[str, tuple[str, None]]] = {"python": ("https://docs.python.org/3", None)}
nitpicky: Final[bool] = True
nitpick_ignore: Final[list[tuple[str, str]]] = []
# anchors that resolve in a browser but linkcheck cannot verify: GitHub renders README heading ids with a
# user-content- prefix added client-side
linkcheck_anchors_ignore_for_url: Final[list[str]] = [r"https://github\.com/.*"]

markdown_http_base: Final[str] = os.environ.get("READTHEDOCS_CANONICAL_URL", "")
# sphinx-markdown-builder cannot translate the label node of sphinx-inline-tabs, so llms.txt lacks tab titles
llms_txt_suppress_unknown_node_warnings: Final[list[str]] = ["label"]


class _RuleEntry(TypedDict):
    code: str
    name: str
    summary: str
    fix: str
    explanation: str


def _generate(docs: Path) -> None:
    # The binary embeds each rule's explanation and its help text, so `simplint rule` and these pages cannot drift.
    if (binary := shutil.which("simplint")) is None:
        msg = "simplint is not on PATH; build the docs through `tox run -e docs`"
        raise FileNotFoundError(msg)
    generated = docs / "_generated"
    (generated / "rules").mkdir(parents=True, exist_ok=True)
    help_text = subprocess.run([binary, "check", "--help"], check=True, capture_output=True, text=True).stdout
    _write(generated / "check-help.txt", help_text)
    listing = subprocess.run([binary, "rules", "--output-format", "json"], check=True, capture_output=True, text=True)
    rules: list[_RuleEntry] = json.loads(listing.stdout)
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
    table: list[str] = []
    for prefix, title in groups.items():
        table.extend([
            f".. list-table:: {title}",
            "   :header-rows: 1",
            "   :widths: 10 25 55 10",
            "",
            "   * - Code",
            "     - Name",
            "     - Checks",
            "     - Fix",
        ])
        for rule in (entry for entry in rules if entry["code"].startswith(prefix)):
            fix = {"always": "🛠️", "sometimes": "🔧", "none": ""}[rule["fix"]]
            summary = pypandoc.convert_text(rule["summary"], "rst", format="markdown").strip()
            table.extend([
                f"   * - :doc:`{rule['code']} <_generated/rules/{rule['code']}>`",
                f"     - {rule['name']}",
                f"     - {summary}",
                f"     - {fix}",
            ])
        table.append("")
    table.extend([
        ".. toctree::",
        "   :hidden:",
        "",
        *(f"   _generated/rules/{rule['code']}" for rule in rules),
        "",
    ])
    _write(generated / "rules-table.inc", "\n".join(table))
    for rule in rules:
        title = f"{rule['name']} ({rule['code']})"
        body = pypandoc.convert_text(rule["explanation"], "rst", format="markdown", extra_args=["--columns=120"])
        page = f"{title}\n{'=' * len(title)}\n\n{fix_notes[rule['fix']]}\n\n{body}"
        _write(generated / "rules" / f"{rule['code']}.rst", page)
    current = {f"{rule['code']}.rst" for rule in rules}
    for stale in (page for page in (generated / "rules").iterdir() if page.name not in current):
        stale.unlink()


def _write(path: Path, content: str) -> None:
    # sphinx-llm reruns this configuration in a second build that reads these files, so rewrite only what changed
    if not path.exists() or path.read_text(encoding="utf-8") != content:
        path.write_text(content, encoding="utf-8")


_generate(Path(__file__).parent)
