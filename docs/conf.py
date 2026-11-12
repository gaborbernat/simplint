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
# the manifest rather than the installed package, so the docs build needs no compiled binary
with (Path(__file__).parents[1] / "Cargo.toml").open("rb") as manifest:
    ver: Final[str] = tomllib.load(manifest)["package"]["version"]
release, version = ver, ".".join(ver.split(".")[:2])
now: Final[datetime] = datetime.now(tz=UTC)
project_copyright: Final[str] = f"2026-{now.year}, {company}"
master_doc: Final[str] = "index"
source_suffix: Final[dict[str, str]] = {".rst": "restructuredtext", ".md": "markdown"}
myst_enable_extensions: Final[list[str]] = ["colon_fence"]
# rule pages draw diagrams from plain ```mermaid fences, the form the terminal output and GitHub also understand
myst_fence_as_directive: Final[list[str]] = ["mermaid"]

html_theme: Final[str] = "furo"
html_title, html_last_updated_fmt = name, now.isoformat()
# both pass WCAG AA (4.5:1) for every token color against their own background
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
        # Furo's grays pass WCAG AA but read washed out in the small sidebar and contents text, which derive from these;
        # its dark palette sets both, so they apply to the light theme only
        "color-foreground-secondary": "#3b3f46",
        "color-foreground-muted": "#4a4f57",
        # a near-black on off-white reads at 15:1 without the glare of pure black on white
        "color-foreground-primary": "#1e2329",
        "color-background-primary": "#fcfcfb",
        # Furo draws pager arrows, link underlines on hover and, through simplint.css, control outlines in this color;
        # its default reaches only 3.6:1
        "color-foreground-border": "#63676f",
    },
    "dark_css_variables": {
        # the logo's yellow, toned down a step so links and the current page do not glare on the navy
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
# Diagrams are drawn once, at build time, by mermaid-cli, which npx fetches at this version, so pages carry finished
# SVGs and load no script. Its headless browser comes from puppeteer, which downloads one when npx installs it, or
# runs the one PUPPETEER_EXECUTABLE_PATH names when PUPPETEER_SKIP_DOWNLOAD is set.
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

# copying a snippet takes the code alone, not the line numbers or console prompts and output
copybutton_exclude: Final[str] = ".linenos, .gp, .go"
exclude_patterns: Final[list[str]] = ["_build", "changelog/*", "_draft.rst"]

extlinks: Final[dict[str, tuple[str, str]]] = {
    "pypi": ("https://pypi.org/project/%s", "%s"),
    "gh": ("https://github.com/%s", "%s"),
}
intersphinx_mapping: Final[dict[str, tuple[str, None]]] = {"python": ("https://docs.python.org/3", None)}
nitpicky: Final[bool] = True
nitpick_ignore: Final[list[tuple[str, str]]] = []
# anchors that resolve in a browser but linkcheck cannot verify: GitHub renders README heading ids with a
# user-content- prefix added client-side, and pyright's single-page docs route on the part after the hash
# cppreference answers automated clients with 403 Forbidden while the page loads in a browser; the changelog links each
# entry's pull request and author profile, which the changelog task builds from real pull requests and GitHub answers
# with 504 from CI runners
linkcheck_ignore: Final[list[str]] = [
    r"https://en\.cppreference\.com/.*",
    r"https://github\.com/gaborbernat/simplint/pull/\d+$",
    r"https://github\.com/[\w-]+$",
]
linkcheck_anchors_ignore_for_url: Final[list[str]] = [
    r"https://github\.com/.*",
    r"https://microsoft\.github\.io/pyright/.*",
]
# documentation hosts such as docs.astral.sh sometimes take longer than the default 30 seconds to answer
linkcheck_timeout: Final[int] = 60

markdown_http_base: Final[str] = os.environ.get("READTHEDOCS_CANONICAL_URL", "")
# sphinx-markdown-builder cannot translate sphinx-inline-tabs labels, sphinx-design card titles or mermaid diagrams,
# so llms.txt lacks them
llms_txt_suppress_unknown_node_warnings: Final[list[str]] = ["label", "PassthroughTextElement", "mermaid"]


# every page's template context, where the edit and view links of a rule page find the file it comes from
html_context: Final[dict[str, dict[str, str]]] = {"rule_sources": rule_pages.generate(Path(__file__).parent)}
