"""Configure the documentation build."""

from __future__ import annotations

import html
import json
import os
import re
import shutil
import subprocess
import sys
import textwrap
from datetime import UTC, datetime
from importlib.metadata import version as metadata_version
from pathlib import Path
from typing import TYPE_CHECKING, Any, Final, TypedDict

if TYPE_CHECKING:
    from docutils import nodes
    from sphinx.application import Sphinx

# sphinx-argparse-cli imports the parser builder that sits next to this file
sys.path.insert(0, str(Path(__file__).parent))

company, name = "gaborbernat", "simplint"
ver: Final[str] = metadata_version("simplint")
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
    "light_logo": "wordmark-light.png",
    "dark_logo": "wordmark-dark.png",
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
    },
    "dark_css_variables": {
        "color-brand-primary": "#ffd43b",
        "color-brand-content": "#ffd43b",
        "color-brand-visited": "#ffd43b",
        "color-background-primary": "#0b1626",
        "color-background-secondary": "#0f1e33",
        "color-background-hover": "#14273f",
        "color-background-border": "#1f3654",
        "color-sidebar-link-text--top-level": "#d6dde8",
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
]
# each diagram colors its nodes from the brand palette, which reads on both Furo themes, so both start from neutral
mermaid_light_theme: Final[str] = "base"
mermaid_dark_theme: Final[str] = "base"
mermaid_height: Final[str] = "auto"
mermaid_init_config: Final[dict[str, bool | dict[str, str]]] = {
    "startOnLoad": False,
    "themeVariables": {"fontSize": "18px"},
}

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
# cppreference answers automated clients with 403 Forbidden while the page loads in a browser
linkcheck_ignore: Final[list[str]] = [r"https://en\.cppreference\.com/.*"]
linkcheck_anchors_ignore_for_url: Final[list[str]] = [
    r"https://github\.com/.*",
    r"https://microsoft\.github\.io/pyright/.*",
]

markdown_http_base: Final[str] = os.environ.get("READTHEDOCS_CANONICAL_URL", "")
# sphinx-markdown-builder cannot translate sphinx-inline-tabs labels, sphinx-design card titles or mermaid diagrams,
# so llms.txt lacks them
llms_txt_suppress_unknown_node_warnings: Final[list[str]] = ["label", "PassthroughTextElement", "mermaid"]


class _RuleEntry(TypedDict):
    code: str
    name: str
    summary: str
    fix: str
    explanation: str


def _generate(docs: Path) -> dict[str, str]:
    # The binary embeds each rule's explanation and its help text, so `simplint rule` and these pages cannot drift.
    if (binary := shutil.which("simplint")) is None:
        msg = "simplint is not on PATH; build the docs through `tox run -e docs`"
        raise FileNotFoundError(msg)
    generated = docs / "_generated"
    (generated / "rules").mkdir(parents=True, exist_ok=True)
    listing = subprocess.run([binary, "rules", "--output-format", "json"], check=True, capture_output=True, text=True)
    rules: list[_RuleEntry] = json.loads(listing.stdout)
    (generated / "groups").mkdir(exist_ok=True)
    _write(generated / "rules-table.inc", _rules_index(rules))
    for prefix, title in _GROUPS.items():
        _write(
            generated / "groups" / f"{prefix}.rst",
            _group_page(title, [r for r in rules if r["code"].startswith(prefix)]),
        )
    for rule in rules:
        _write(generated / "rules" / f"{rule['code']}.md", _rule_page(rule))
    current = {f"{rule['code']}.md" for rule in rules}
    for stale in (page for page in (generated / "rules").iterdir() if page.name not in current):
        stale.unlink()
    # each explanation sits next to its rule, in the directory of the rule's family
    root = docs.parent
    return {
        rule["code"]: next(root.glob(f"src/rules/*/{rule['name'].replace('-', '_')}.md")).relative_to(root).as_posix()
        for rule in rules
    }


_GROUPS: Final[dict[str, str]] = {
    "SMP0": "Comments, docstrings and structure",
    "SMP1": "Modules, typing and exports",
    "SMP2": "Tests",
}
_FIX_BADGES: Final[dict[str, tuple[str, str]]] = {
    "Always": ("fix-always", "🛠️ Fix always available"),
    "Sometimes": ("fix-sometimes", "🔧 Fix sometimes available"),
    "None": ("fix-none", "No fix"),
}


def _rules_index(rules: list[_RuleEntry]) -> str:
    groups = "".join(f'<option value="{prefix}">{html.escape(title)}</option>' for prefix, title in _GROUPS.items())
    rows: list[str] = []
    for prefix, title in _GROUPS.items():
        rows.append(f'<tr class="group-row" data-group="{prefix}"><td colspan="4">{html.escape(title)}</td></tr>')
        for rule in (entry for entry in rules if entry["code"].startswith(prefix)):
            badge_class, badge_text = _FIX_BADGES[rule["fix"]]
            fix = (
                ""
                if rule["fix"] == "None"
                else f'<span class="badge {badge_class}">{badge_text.split(" ", 1)[0]}</span>'
            )
            searchable = html.escape(f"{rule['code']} {rule['name']} {rule['summary']}".lower(), quote=True)
            rows.append(
                f'<tr class="rule-row" id="{rule["name"]}" data-group="{prefix}" data-fix="{rule["fix"]}"'
                f' data-text="{searchable}">'
                f'<td class="rule-code" id="{rule["code"]}">'
                f'<a href="_generated/rules/{rule["code"]}.html">{rule["code"]}</a></td>'
                f'<td class="rule-name">{rule["name"]}</td>'
                f'<td class="rule-summary">{_inline_code(rule["summary"])}</td>'
                f'<td class="rule-fix">{fix}</td></tr>'
            )
    body = "\n".join(rows)
    markup = (
        '<div class="rule-filter">'
        '<input id="rule-search" type="search" placeholder="Filter by code, name or description (press /)"'
        ' aria-label="Filter rules">'
        f'<select id="rule-group" aria-label="Group"><option value="">All groups</option>{groups}</select>'
        '<select id="rule-fix" aria-label="Fix availability"><option value="">Any fix</option>'
        '<option value="Always">Fix always available</option><option value="Sometimes">Fix sometimes available</option>'
        '<option value="None">No fix</option></select>'
        '<span id="rule-count" class="rule-count"></span></div>'
        '<table class="rules"><thead><tr><th>Code</th><th class="rule-name">Name</th><th>Checks</th><th>Fix</th>'
        f"</tr></thead><tbody>\n{body}\n</tbody></table>"
        '<p id="rule-empty" class="rule-empty" hidden>No rule matches these filters.</p>'
    )
    # the sidebar shows this hidden tree, so a rule page lists its siblings for switching between rules
    groups_tree = "\n".join(f"   {title} <_generated/groups/{prefix}>" for prefix, title in _GROUPS.items())
    return f".. raw:: html\n\n{textwrap.indent(markup, '   ')}\n\n.. toctree::\n   :hidden:\n\n{groups_tree}\n"


def _group_page(title: str, rules: list[_RuleEntry]) -> str:
    entries = "\n".join(f"   {rule['code']} · {rule['name']} <../rules/{rule['code']}>" for rule in rules)
    return f"{title}\n{'=' * len(title)}\n\n.. toctree::\n   :titlesonly:\n\n{entries}\n"


def _inline_code(text: str) -> str:
    return re.sub(r"`([^`]+)`", r"<code>\1</code>", html.escape(text))


def _rule_page(rule: _RuleEntry) -> str:
    # rule pages stay Markdown, rendered by MyST, so links on inline code and fenced diagrams survive as written
    title = f"{rule['name']} ({rule['code']})"
    badge_class, badge_text = _FIX_BADGES[rule["fix"]]
    group = next(title for prefix, title in _GROUPS.items() if rule["code"].startswith(prefix))
    badges = (
        f'<div class="badges"><span class="badge code">{rule["code"]}</span>'
        f'<span class="badge {badge_class}">{badge_text}</span><span class="badge">{html.escape(group)}</span>'
        '<a class="badge" href="../../rules.html">All rules</a></div>'
    )
    explanation = re.sub(r"\]\((SMP\d+)\.html\)", r"](\1.md)", rule["explanation"])
    body: list[str] = []
    for section in (part for part in re.split(r"^## ", explanation, flags=re.MULTILINE) if part.strip()):
        heading, _, content = section.partition("\n")
        body.extend([f"## {heading}", ""])
        before, marker, after = content.partition("\nUse instead:\n")
        if heading == "Example" and marker:
            body.extend(["::::{grid} 1", ":gutter: 2", ""])
            for label, css, part in (("Flagged", "bad", before), ("Preferred", "good", after)):
                body.extend([
                    f":::{{grid-item-card}} {label}",
                    f":class-card: example {css}",
                    "",
                    _numbered(part),
                    ":::",
                    "",
                ])
            body.extend(["::::", ""])
        else:
            body.extend([content.strip(), ""])
    # both the code and the name are labels, so other pages can link a rule by either
    targets = [f"({rule['code']})=", f"({rule['name']})="]
    return "\n".join([*targets, f"# {title}", "", "```{raw} html", badges, "```", "", *body])


def _numbered(markdown: str) -> str:
    # example snippets are short and read line by line, so number them; the stylesheet wraps long lines
    return markdown.strip().replace("```python\n", "```{code-block} python\n:linenos:\n\n")


def _write(path: Path, content: str) -> None:
    # sphinx-llm reruns this configuration in a second build that reads these files, so rewrite only what changed
    if not path.exists() or path.read_text(encoding="utf-8") != content:
        path.write_text(content, encoding="utf-8")


def setup(app: Sphinx) -> None:
    app.connect("html-page-context", _point_generated_pages_at_their_source)


def _point_generated_pages_at_their_source(
    app: Sphinx, pagename: str, templatename: str, context: dict[str, Any], doctree: nodes.document | None
) -> None:
    # Generated pages have no file under docs/, so the theme's view and edit icons would point nowhere: a rule page
    # comes from its explanation in the crate, the group pages from this configuration.
    del app, templatename, doctree
    if pagename.startswith("_generated/rules/"):
        source = context["rule_sources"][pagename.rsplit("/", 1)[1]]
    elif pagename.startswith("_generated/"):
        source = "docs/conf.py"
    else:
        return
    context["theme_source_edit_link"] = f"https://github.com/gaborbernat/simplint/edit/main/{source}"
    context["theme_source_view_link"] = f"https://github.com/gaborbernat/simplint/blob/main/{source}"


# every page's template context, where the edit and view links of a rule page find the file it comes from
html_context: Final[dict[str, dict[str, str]]] = {"rule_sources": _generate(Path(__file__).parent)}
