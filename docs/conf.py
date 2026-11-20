"""Configure the documentation build."""

from __future__ import annotations

import os
import shutil
import sys
import tomllib
from datetime import UTC, datetime
from pathlib import Path
from typing import Final

# sphinx-argparse-cli imports the parser builder that sits next to this file
sys.path.insert(0, str(Path(__file__).parent))

from _ext import diagrams, rule_pages  # ruff: ignore[import-private-name] # the build's own modules

company, name = "gaborbernat", "simplint"
# Reading the manifest lets Sphinx build without installing the binary.
with (Path(__file__).parents[1] / "Cargo.toml").open("rb") as manifest:
    ver: Final[str] = tomllib.load(manifest)["package"]["version"]
release, version = ver, ".".join(ver.split(".")[:2])
now: Final[datetime] = datetime.now(tz=UTC)
project_copyright: Final[str] = f"2026-{now.year}, {company}"
master_doc: Final[str] = "index"
source_suffix: Final[dict[str, str]] = {".rst": "restructuredtext", ".md": "markdown"}
myst_enable_extensions: Final[list[str]] = ["colon_fence"]
# Rule pages use the same Mermaid fences on GitHub and in Sphinx.
myst_fence_as_directive: Final[list[str]] = ["mermaid"]

html_theme: Final[str] = "furo"
html_title, html_last_updated_fmt = name, now.isoformat()
pygments_style, pygments_dark_style = "xcode", "github-dark"
html_static_path: Final[list[str]] = ["_static"]
html_css_files: Final[list[str]] = ["simplint.css"]
html_js_files: Final[list[str]] = ["rules.js"]
html_favicon: Final[str] = "_static/favicon.svg"
templates_path: Final[list[str]] = ["_templates"]
html_theme_options: Final[dict[str, str | bool | dict[str, str]]] = {
    "source_repository": "https://github.com/gaborbernat/simplint",
    "source_branch": "main",
    "source_directory": "docs/",
    "light_logo": "wordmark-light.webp",
    "dark_logo": "wordmark-dark.webp",
    "sidebar_hide_name": True,
    "light_css_variables": {
        "color-brand-primary": "#2b6599",
        "color-brand-content": "#2b6599",
        "color-brand-visited": "#2b6599",
        "font-stack": "Inter, ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif",
        "font-stack--monospace": "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, monospace",
        # top-level entries take the text color, so only the branch holding the current page shows the brand color
        "color-sidebar-link-text--top-level": "#1f2937",
        # Darken secondary text in the light theme's sidebar and table of contents.
        "color-foreground-secondary": "#3b3f46",
        "color-foreground-muted": "#4a4f57",
        "color-foreground-primary": "#1e2329",
        "color-background-primary": "#fcfcfb",
        # Pager arrows, hover underlines and control outlines share this color.
        "color-foreground-border": "#63676f",
    },
    "dark_css_variables": {
        # Use a muted version of the logo's yellow for links on the dark background.
        "color-brand-primary": "#f0c94a",
        "color-brand-content": "#f0c94a",
        "color-brand-visited": "#f0c94a",
        "color-background-primary": "#0b1626",
        "color-background-secondary": "#0f1e33",
        "color-background-hover": "#14273f",
        "color-background-border": "#1f3654",
        "color-sidebar-link-text--top-level": "#d6dde8",
        "color-foreground-border": "#8a96a8",
    },
}

extensions: Final[list[str]] = [
    "sphinx_llm.txt",
    "sphinx.ext.extlinks",
    "sphinx.ext.intersphinx",
    "myst_parser",
    "sphinx_argparse_cli",
    "sphinx_inline_tabs",
    "sphinx_copybutton",
    "sphinx_design",
    "sphinxcontrib.mermaid",
    "_ext.diagrams",
    "_ext.page_shape",
]
# Build SVGs with the pinned Mermaid CLI so readers can view diagrams without running Mermaid.
if shutil.which("npx") is None:
    msg = "the docs draw their diagrams with mermaid-cli, run through npx: install Node.js and put npx on PATH"
    raise RuntimeError(msg)
mermaid_output_format: Final[str] = "svg"
mermaid_cmd: Final[str] = "npx --yes --package=@mermaid-js/mermaid-cli@12.2.0 mmdc"
config_path, puppeteer_path = diagrams.write_settings()
mermaid_sequence_config: Final[str] = str(config_path)
mermaid_params: Final[list[str]] = [
    "--backgroundColor",
    "transparent",
    "--puppeteerConfigFile",
    str(puppeteer_path),
]

copybutton_exclude: Final[str] = ".linenos, .gp, .go"
exclude_patterns: Final[list[str]] = ["_build", "changelog/*", "_draft.rst"]

extlinks: Final[dict[str, tuple[str, str]]] = {
    "pypi": ("https://pypi.org/project/%s", "%s"),
    "gh": ("https://github.com/%s", "%s"),
}
intersphinx_mapping: Final[dict[str, tuple[str, None]]] = {"python": ("https://docs.python.org/3", None)}
nitpicky: Final[bool] = True
nitpick_ignore: Final[list[tuple[str, str]]] = []
# GitHub and Pyright handle these fragments in the browser. The changelog links to individual GitHub PRs and
# profiles. Those links and the two reference sites below have returned errors to automated requests.
linkcheck_ignore: Final[list[str]] = [
    r"https://en\.cppreference\.com/.*",
    r"https://docutils\.sourceforge\.io/.*",
    r"https://github\.com/gaborbernat/simplint/pull/\d+$",
    r"https://github\.com/[\w-]+$",
]
linkcheck_anchors_ignore_for_url: Final[list[str]] = [
    r"https://github\.com/.*",
    r"https://microsoft\.github\.io/pyright/.*",
]
# Give slow documentation hosts more time to respond.
linkcheck_timeout: Final[int] = 60

markdown_http_base: Final[str] = os.environ.get("READTHEDOCS_CANONICAL_URL", "")
# sphinx-markdown-builder cannot translate sphinx-inline-tabs labels, sphinx-design card titles or mermaid diagrams,
# so llms.txt lacks them
llms_txt_suppress_unknown_node_warnings: Final[list[str]] = ["label", "PassthroughTextElement", "mermaid"]


# Generated rule pages link to the Markdown source in the crate.
html_context: Final[dict[str, dict[str, str]]] = {"rule_sources": rule_pages.generate(Path(__file__).parent)}
