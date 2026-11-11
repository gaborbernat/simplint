"""Draw the diagrams of the documentation once per build with mermaid-cli, and inline the drawings into the pages."""

from __future__ import annotations

import json
import re
import shlex
import shutil
import subprocess  # ruff: ignore[suspicious-subprocess-import] # runs the pinned mermaid-cli
from functools import partial
from hashlib import sha1, sha256
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import TYPE_CHECKING, Final

from _ext.rule_pages import remove_stale, write_if_changed  # ruff: ignore[import-private-name] # build modules
from sphinxcontrib.mermaid import mermaid

if TYPE_CHECKING:
    from docutils import nodes
    from sphinx.application import Sphinx
    from sphinx.builders import Builder

_DIAGRAMS: Final[Path] = Path(__file__).parents[1] / "_derived" / "mermaid"
# the labels use a face every viewer has, or one with its widths, so the text fits the boxes drawn around it
_DIAGRAM_FONT: Final[dict[str, str]] = {
    "fontFamily": "Arial, 'Liberation Sans', Helvetica, sans-serif",
    "fontSize": "18px",
}
# each diagram colors its nodes from the brand palette, which reads on both Furo themes, so it starts from neutral;
# mermaid sizes each box to its label in the plain weight, so a bold label would wrap
_DIAGRAM_CONFIG: Final[str] = json.dumps({
    "theme": "base",
    "fontFamily": _DIAGRAM_FONT["fontFamily"],
    "themeVariables": _DIAGRAM_FONT,
    "themeCSS": ".edgeLabel { font-size: 0.85em; }",
})

# The extension passes this file as the mermaid configuration and names each drawing after its path, and reuses a
# drawing it finds under that name; a path that holds the content's hash redraws every diagram when the configuration
# changes.
CONFIG_PATH: Final[Path] = _DIAGRAMS / f"config-{sha256(_DIAGRAM_CONFIG.encode()).hexdigest()[:12]}.json"
PUPPETEER_PATH: Final[Path] = _DIAGRAMS / "puppeteer.json"


def write_settings() -> None:
    # The browser renders only the diagrams of these pages, so it needs no sandbox, which containers often cannot give
    # it. Read the Docs builds in a small container: its /dev/shm is too small for Chrome, and a slow start there can
    # outlast puppeteer's default 30 s wait.
    _DIAGRAMS.mkdir(parents=True, exist_ok=True)
    write_if_changed(CONFIG_PATH, _DIAGRAM_CONFIG)
    puppeteer = {"args": ["--no-sandbox", "--disable-dev-shm-usage"], "timeout": 180_000}
    write_if_changed(PUPPETEER_PATH, json.dumps(puppeteer))
    remove_stale(_DIAGRAMS, {CONFIG_PATH.name, PUPPETEER_PATH.name})


def setup(app: Sphinx) -> dict[str, bool]:
    app.connect("write-started", _draw_diagrams)
    app.connect("html-page-context", _inline_diagrams, priority=601)
    return {"parallel_read_safe": True, "parallel_write_safe": True}


def _draw_diagrams(app: Sphinx, builder: Builder) -> None:
    # The extension starts a browser for each diagram it writes, and parallel writers start several at once, more than a
    # small container can launch in time. One mermaid-cli run draws the diagrams before the pages are written, under the
    # names the extension looks for, so the extension reuses them and starts no browser.
    if builder.format != "html":
        return
    images = Path(builder.outdir) / builder.imagedir
    pending: dict[Path, str] = {}
    for docname in sorted(app.env.found_docs):
        for node in app.env.get_doctree(docname).findall(mermaid):
            key = f"{node['code']}{node['options']}{app.config.mermaid_sequence_config}".encode()
            if not (target := images / f"mermaid-{sha1(key, usedforsecurity=False).hexdigest()}.svg").exists():
                pending[target] = node["code"]
    if not pending:
        return
    images.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory() as folder:
        work = Path(folder)
        (work / "diagrams.md").write_text(
            "".join(f"```mermaid\n{code}\n```\n\n" for code in pending.values()), encoding="utf-8"
        )
        command = [
            *shlex.split(app.config.mermaid_cmd),
            *app.config.mermaid_params,
            "--configFile",
            app.config.mermaid_sequence_config,
        ]
        command += ["--input", str(work / "diagrams.md"), "--output", str(work / "drawn.md")]
        run = subprocess.run(command, capture_output=True, text=True, check=False)  # ruff: ignore[subprocess-without-shell-equals-true] # the pinned command above
        if run.returncode:
            msg = f"mermaid-cli could not draw the diagrams:\n{run.stderr}{run.stdout}"
            raise RuntimeError(msg)
        # mermaid-cli numbers the drawings of a Markdown file in the order of its diagrams
        for index, target in enumerate(pending, start=1):
            shutil.move(work / f"drawn-{index}.svg", target)


def _inline_diagrams(
    app: Sphinx, pagename: str, templatename: str, context: dict[str, str], doctree: nodes.document | None
) -> None:
    del pagename, templatename, doctree
    if "body" in context:
        diagrams = Path(app.outdir) / app.builder.imagedir
        context["body"] = _DIAGRAM.sub(partial(_inline_diagram, diagrams), context["body"])


def _inline_diagram(diagrams: Path, match: re.Match[str]) -> str:
    # In the page, rather than behind an object element, a diagram takes its size from its viewBox as the page lays out,
    # and the stylesheet colors its edges and labels for the theme. Each keeps ids of its own, as a page may hold two.
    svg = (diagrams / match["name"]).read_text(encoding="utf-8").replace("my-svg", match["name"].removesuffix(".svg"))
    return f'<div class="mermaid">{svg}</div>'


_DIAGRAM: Final[re.Pattern[str]] = re.compile(
    r'<object data="[^"]*?(?P<name>mermaid-[0-9a-f]+\.svg)" type="image/svg\+xml">.*?</object>', re.DOTALL
)
