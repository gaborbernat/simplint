"""Configure the documentation build."""

from __future__ import annotations

import html
import json
import os
import posixpath
import re
import shlex
import shutil
import subprocess  # ruff: ignore[suspicious-subprocess-import] # runs the pinned mermaid-cli
import sys
import textwrap
import tomllib
from datetime import UTC, datetime
from functools import partial
from hashlib import sha1, sha256
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import TYPE_CHECKING, Final, NotRequired, TypedDict

from docutils import nodes
from sphinx_design.dropdown import dropdown_main
from sphinxcontrib.mermaid import mermaid

if TYPE_CHECKING:
    from sphinx.application import Sphinx
    from sphinx.builders import Builder

# sphinx-argparse-cli imports the parser builder that sits next to this file
sys.path.insert(0, str(Path(__file__).parent))

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
]
# Diagrams are drawn once, at build time, by mermaid-cli, which npx fetches at this version, so pages carry finished
# SVGs and load no script. Its headless browser comes from puppeteer, which downloads one when npx installs it, or
# runs the one PUPPETEER_EXECUTABLE_PATH names when PUPPETEER_SKIP_DOWNLOAD is set.
if shutil.which("npx") is None:
    msg = "the docs draw their diagrams with mermaid-cli, run through npx: install Node.js and put npx on PATH"
    raise RuntimeError(msg)
_DIAGRAMS: Final[Path] = Path(__file__).parent / "_generated" / "mermaid"
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
mermaid_output_format: Final[str] = "svg"
mermaid_cmd: Final[str] = "npx --yes --package=@mermaid-js/mermaid-cli@12.2.0 mmdc"
# The extension passes this file as the mermaid configuration and names each drawing after its path, and reuses a
# drawing it finds under that name; a path that holds the content's hash redraws every diagram when the configuration
# changes.
mermaid_sequence_config: Final[str] = str(
    _DIAGRAMS / f"config-{sha256(_DIAGRAM_CONFIG.encode()).hexdigest()[:12]}.json"
)
mermaid_params: Final[list[str]] = [
    "--backgroundColor",
    "transparent",
    "--puppeteerConfigFile",
    str(_DIAGRAMS / "puppeteer.json"),
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


class _Fence(TypedDict):
    line: int
    end_line: int
    source: str


class _Block(_Fence):
    role: str
    language: str
    file: str
    named: bool


class _Example(TypedDict):
    section: str | None
    line: int
    end_line: int
    config: _Fence | None
    blocks: list[_Block]


class _Examples(TypedDict, total=False):
    parsed: list[_Example]
    error: str


class _RuleEntry(TypedDict):
    code: str
    name: str
    summary: str
    fix: str
    explanation: str
    examples: _Examples
    family: str
    group: str
    short_group: str
    cross_file: bool


def _generate(docs: Path) -> dict[str, str]:
    # `simplint rules --output-format json`, committed and kept equal to the binary's by `tests/docs_data.rs`, so the
    # pages and `simplint rule` cannot drift
    generated = docs / "_generated"
    (generated / "rules").mkdir(parents=True, exist_ok=True)
    _DIAGRAMS.mkdir(exist_ok=True)
    _write(Path(mermaid_sequence_config), _DIAGRAM_CONFIG)
    # The browser renders only the diagrams of these pages, so it needs no sandbox, which containers often cannot give
    # it. Read the Docs builds in a small container: its /dev/shm is too small for Chrome, and a slow start there can
    # outlast puppeteer's default 30 s wait.
    puppeteer = {"args": ["--no-sandbox", "--disable-dev-shm-usage"], "timeout": 180_000}
    _write(_DIAGRAMS / "puppeteer.json", json.dumps(puppeteer))
    rules: list[_RuleEntry] = json.loads((docs / "_data" / "rules.json").read_text(encoding="utf-8"))
    # each group's title and short title by its family, the directory its rules live in, which names its pages since
    # titles have spaces; the binary lists rules by code, so each group appears where its first rule does
    groups = {rule["family"]: (rule["group"], rule["short_group"]) for rule in rules}
    (generated / "groups").mkdir(exist_ok=True)
    _write(generated / "rules-table.inc", _rules_index(rules, groups))
    for family, (title, _) in groups.items():
        _write(generated / "groups" / f"{family}.rst", _group_page(title, rules))
    for rule in rules:
        _write(generated / "rules" / f"{rule['code']}.md", _rule_page(rule))
    _remove_stale(generated / "rules", {f"{rule['code']}.md" for rule in rules})
    _remove_stale(generated / "groups", {f"{family}.rst" for family in groups})
    _remove_stale(_DIAGRAMS, {Path(mermaid_sequence_config).name, "puppeteer.json"})
    # each explanation sits next to its rule, in the directory of the rule's family
    root = docs.parent
    explanations: dict[str, str] = {}
    for rule in rules:
        pattern = f"src/rules/*/{rule['name'].replace('-', '_')}.md"
        if (found := next(root.glob(pattern), None)) is None:
            msg = f"no explanation for {rule['code']}: expected a file matching {pattern}"
            raise FileNotFoundError(msg)
        explanations[rule["code"]] = found.relative_to(root).as_posix()
    return explanations


def _remove_stale(folder: Path, current: set[str]) -> None:
    # a page whose rule or group is gone would still build, under a name that no longer exists
    for stale in (page for page in folder.iterdir() if page.name not in current):
        stale.unlink()


_FIX_BADGES: Final[dict[str, tuple[str, str]]] = {
    "Always": ("fix-always", "🛠️ Fix always available"),
    "Sometimes": ("fix-sometimes", "🔧 Fix sometimes available"),
    "None": ("fix-none", "No fix"),
}


def _rules_index(rules: list[_RuleEntry], groups: dict[str, tuple[str, str]]) -> str:
    options = "".join(
        f'<option value="{family}">{html.escape(group)}</option>' for family, (group, _) in groups.items()
    )
    rows: list[str] = []
    for family, (group, _) in groups.items():
        rows.append(f'<tr class="group-row" data-group="{family}"><td colspan="5">{html.escape(group)}</td></tr>')
        rows.extend(_rule_row(rule) for rule in rules if rule["family"] == family)
    body = "\n".join(rows)
    markup = (
        '<div class="rule-filter">'
        # as wide as the longest rule name, the longest text worth typing into it
        '<input id="rule-search" type="search" placeholder="Filter rules (press /)" aria-label="Filter rules"'
        f' style="--rule-search-chars: {max(len(rule["name"]) for rule in rules)}">'
        f'<select id="rule-group" aria-label="Group"><option value="">All groups</option>{options}</select>'
        '<select id="rule-fix" aria-label="Fix availability"><option value="">Any fix</option>'
        '<option value="Always">Fix always available</option><option value="Sometimes">Fix sometimes available</option>'
        '<option value="None">No fix</option></select>'
        '<select id="rule-scope" aria-label="Files read"><option value="">All rules</option>'
        '<option value="cross-file">Cross-file only</option>'
        '<option value="single-file">Single-file only</option></select>'
        # the count, like the rows, is final in the markup, so a page without filters needs no script to settle
        f'<span id="rule-count" class="rule-count">{len(rules)} rules</span></div>'
        '<table class="rules"><thead><tr><th class="rule-code">Code</th><th class="rule-name">Name</th>'
        '<th class="rule-summary">Checks</th><th class="rule-fix">Fix</th>'
        '<th class="rule-scope"><a href="explanation/how-it-works.html#cross-file-rules"'
        ' title="Other files of the run can change what the rule reports">Cross-file</a></th>'
        f"</tr></thead><tbody>\n{body}\n</tbody></table>"
        '<p id="rule-empty" class="rule-empty" hidden>No rule matches these filters.</p>'
    )
    # the sidebar shows this hidden tree, so a rule page lists its siblings for switching between rules; it has room
    # for one line per entry, so each group shows its short title there
    groups_tree = "\n".join(f"   {short} <_generated/groups/{family}>" for family, (_, short) in groups.items())
    return f".. raw:: html\n\n{textwrap.indent(markup, '   ')}\n\n.. toctree::\n   :hidden:\n\n{groups_tree}\n"


def _rule_row(rule: _RuleEntry) -> str:
    badge_class, badge_text = _FIX_BADGES[rule["fix"]]
    fix = (
        ""
        if rule["fix"] == "None"
        else f'<span class="badge {badge_class}" title="{badge_text}" aria-label="{badge_text}">'
        f"{badge_text.split(' ', 1)[0]}</span>"
    )
    searchable = html.escape(f"{rule['code']} {rule['name']} {rule['summary']}".lower(), quote=True)
    summary = re.sub(r"`([^`]+)`", r"<code>\1</code>", html.escape(rule["summary"]))
    return (
        f'<tr class="rule-row" id="{rule["name"]}" data-group="{rule["family"]}" data-fix="{rule["fix"]}"'
        f' data-scope="{"cross-file" if rule["cross_file"] else "single-file"}"'
        f' data-text="{searchable}">'
        f'<td class="rule-code" id="{rule["code"]}">'
        f'<a href="_generated/rules/{rule["code"]}.html">{rule["code"]}</a></td>'
        f'<td class="rule-name">{rule["name"]}</td>'
        f'<td class="rule-summary">{summary}</td>'
        f'<td class="rule-fix">{fix}</td>'
        f'<td class="rule-scope">{"✓" if rule["cross_file"] else "✗"}</td></tr>'
    )


def _group_page(group: str, rules: list[_RuleEntry]) -> str:
    entries = "\n".join(
        f"   {rule['code']} · {rule['name']} <../rules/{rule['code']}>" for rule in rules if rule["group"] == group
    )
    return f"{group}\n{'=' * len(group)}\n\n.. toctree::\n   :titlesonly:\n\n{entries}\n"


def _rule_page(rule: _RuleEntry) -> str:
    # rule pages stay Markdown, rendered by MyST, so links on inline code and fenced diagrams survive as written
    if "error" in (examples := rule["examples"]):
        msg = f"{rule['code']}: {examples['error']}"
        raise ValueError(msg)
    title = f"{rule['name']} ({rule['code']})"
    badge_class, badge_text = _FIX_BADGES[rule["fix"]]
    # links to the page that explains what reading other files means for a rule
    cross_file = (
        '<a class="badge cross-file" href="../../explanation/how-it-works.html#cross-file-rules"'
        ' title="Other files of the run can change what this rule reports">🔗 Cross-file</a>'
        if rule["cross_file"]
        else ""
    )
    badges = (
        f'<div class="badges"><span class="badge code">{rule["code"]}</span>'
        f'<span class="badge {badge_class}">{badge_text}</span><span class="badge">{html.escape(rule["group"])}</span>'
        f"{cross_file}"
        '<a class="badge" href="../../rules.html">All rules</a></div>'
    )
    explanation = re.sub(r"\]\((SMP\d+)\.html\)", r"](\1.md)", rule["explanation"])
    # links to the settings and how-to pages stay absolute in the binary, for `simplint rule`; built, they resolve to
    # the labels on those pages, so each version of these docs links its own
    explanation = re.sub(r"\]\(https://simplint\.readthedocs\.io/en/latest/[\w/.-]+#", "](#", explanation)
    if re.search(r"https?://simplint\.readthedocs\.io/", explanation):
        msg = f"{rule['code']} links to a simplint.readthedocs.io page that has no label to link to"
        raise ValueError(msg)
    lines = explanation.split("\n")
    by_line = {example["line"]: example for example in examples["parsed"]}
    starts = [index for index, line in enumerate(lines) if line.startswith("## ")]
    # an explanation opens with a summary paragraph that has no heading of its own
    summary = "\n".join(lines[: starts[0]] if starts else lines).strip()
    body = [summary, ""] if summary else []
    for start, stop in zip(starts, [*starts[1:], len(lines)], strict=True):
        heading = lines[start].removeprefix("## ")
        body.extend([f"## {heading}", "", *_section_body(heading, lines, start + 1, stop, by_line)])
    # both the code and the name are labels, so other pages can link a rule by either
    targets = [f"({rule['code']})=", f"({rule['name']})="]
    return "\n".join([*targets, f"# {title}", "", "```{raw} html", badges, "```", "", *body])


def _section_body(heading: str, lines: list[str], start: int, stop: int, by_line: dict[int, _Example]) -> list[str]:
    marker = next((index for index in range(start, stop) if lines[index] == "Use instead:"), None)
    if heading != "Example" or marker is None:
        return [_examples(lines, start, stop, by_line, settings=heading == "Configuration").strip(), ""]
    body = ["::::{grid} 1", ":gutter: 2", ""]
    for label, css, (begin, finish) in (
        ("Flagged", "bad", (start, marker)),
        ("Preferred", "good", (marker + 1, stop)),
    ):
        body.extend([
            f":::{{grid-item-card}} {label}",
            f":class-card: example {css}",
            "",
            _numbered(lines, begin, finish, by_line),
            ":::",
            "",
        ])
    return [*body, "::::", ""]


def _body(lines: list[str], fence: _Fence) -> list[str]:
    # the lines between a fence's opening and closing lines, which the page keeps as written
    return lines[fence["line"] : fence["end_line"] - 1]


def _indent(lines: list[str], example: _Example) -> str:
    # the fence indentation, which the dropdown keeps so it sits inside the list item
    line = lines[example["line"] - 1]
    return line[: len(line) - len(line.lstrip())]


def _own_config(example: _Example) -> _Fence | None:
    # an Example section config holds for the examples after it, but only the example it opens shows it
    config = example["config"]
    return config if config is not None and config["line"] == example["line"] else None


def _examples(lines: list[str], start: int, stop: int, by_line: dict[int, _Example], *, settings: bool) -> str:
    # Examples with no prose between them form a run, rendered together so a reported one can sit beside its accepted
    # one. Only the Active on cases pair a reported example with its accepted one.
    out: list[str] = []
    run: list[_Example] = []
    index = start
    active = False
    while index < stop:
        if (example := by_line.get(index + 1)) is not None:
            run.append(example)
            out.extend(_gap_lines(lines, example))
            index = example["end_line"]
            continue
        line = lines[index]
        if line.strip():
            out.extend(_render(run, lines, settings=settings, pair=active))
            run = []
        if line.startswith("### "):
            active = line == "### Active on"
        out.append(line)
        index += 1
    # a section ends with the newline that precedes the next heading, so the text after its last line is an empty line
    if stop < len(lines):
        out.append("")
    out.extend(_render(run, lines, settings=settings, pair=active))
    return "\n".join(out)


def _gap_lines(lines: list[str], example: _Example) -> list[str]:
    # the blank lines between an example's fences, which the page keeps where they are
    fences = [*([config] if (config := _own_config(example)) is not None else []), *example["blocks"]]
    gaps: list[str] = []
    previous = example["line"] - 1
    for fence in fences:
        gaps.extend(lines[previous : fence["line"] - 1])
        previous = fence["end_line"]
    return gaps


def _render(run: list[_Example], lines: list[str], *, settings: bool, pair: bool) -> list[str]:
    out: list[str] = []
    index = 0
    while index < len(run):
        if pair and index + 1 < len(run) and _pairs(run[index], run[index + 1]):
            out.extend(_pair(run[index], run[index + 1], lines))
            index += 2
        else:
            out.extend(_dropdown(run[index], lines, settings=settings))
            index += 1
    return out


# a reported example followed by an accepted one under the same configuration reads best side by side
def _pairs(reported: _Example, accepted: _Example) -> bool:
    shown = ({block["role"] for block in example["blocks"]} - {"context"} for example in (reported, accepted))
    return list(shown) == [{"report"}, {"ok"}] and _config_source(reported) == _config_source(accepted)


def _config_source(example: _Example) -> str | None:
    return None if (config := example["config"]) is None else config["source"]


def _pair(reported: _Example, accepted: _Example, lines: list[str]) -> list[str]:
    indent = _indent(lines, reported)
    out = [
        f"{indent}::::{{dropdown}} Example: reported and not",
        f"{indent}:class-container: example example-pair",
        "",
    ]
    if (config := reported["config"]) is not None:
        out.extend(_code_block(indent, "toml", "`pyproject.toml`", _body(lines, config)))
    # captioned like a fix's before and after
    for example, side in ((reported, "reported"), (accepted, "not reported")):
        out.extend([f"{indent}:::{{container}} example-side", ""])
        for block in example["blocks"]:
            label = f"`{block['file']}`" if block["role"] == "context" else _caption(block, side)
            out.extend(_code_block(indent, block["language"], label, _body(lines, block)))
        out.extend([f"{indent}:::", ""])
    return [*out, f"{indent}::::", ""]


def _caption(block: _Block, side: str) -> str:
    return f"`{block['file']}` {side}" if block["named"] else side.capitalize()


def _dropdown(example: _Example, lines: list[str], *, settings: bool) -> list[str]:
    roles = {block["role"] for block in example["blocks"]}
    config = example["config"]
    # an example elsewhere may need a setting to run, as the rst rules do, yet it shows a case, not the setting
    if settings and config is not None:
        title = "configuration"
    elif "fixed" in roles:
        title = "fix"
    elif "report" in roles:
        title = "reported"
    else:
        title = "not reported"
    indent = _indent(lines, example)
    # the role class lets the stylesheet and rules.js find each example and tell its kind apart
    kind = title.replace(" ", "-")
    out = [f"{indent}::::{{dropdown}} Example: {title}", f"{indent}:class-container: example example-{kind}", ""]
    if config is not None:
        out.extend(_code_block(indent, "toml", "`pyproject.toml`", _body(lines, config)))
    for block in example["blocks"]:
        body = _body(lines, block)
        # a lone example.py needs no name; a fix shows its before and after side by side
        file = f"`{block['file']}`" if block["named"] else ""
        if "fixed" not in roles:
            out.extend(_code_block(indent, block["language"], file, body))
            continue
        side = "after" if block["role"] == "fixed" else "before"
        out.append(f"{indent}:::{{container}} example-side")
        out.extend(["", *_code_block(indent, block["language"], _caption(block, side), body)])
        out.extend([f"{indent}:::", ""])
    return [*out, f"{indent}::::", ""]


# an Example section shows each example as a card: its config is not numbered, and context files are left out
def _numbered(lines: list[str], start: int, stop: int, by_line: dict[int, _Example]) -> str:
    out: list[str] = []
    skip_blank = False
    index = start
    while index < stop:
        if (example := by_line.get(index + 1)) is not None:
            cards, skip_blank = _cards(lines, example, skip_blank=skip_blank)
            out.extend(cards)
            index = example["end_line"]
            continue
        if lines[index].strip():
            skip_blank = False
        if not skip_blank:
            out.append(lines[index])
        index += 1
    return "\n".join(out).strip()


# A context file only sets up its example for the test, so the card leaves it out with the blank lines after it. The
# returned flag says whether the blank lines after the example still belong to such a file.
def _cards(lines: list[str], example: _Example, *, skip_blank: bool) -> tuple[list[str], bool]:
    out: list[str] = []
    previous = example["line"] - 1
    if (config := _own_config(example)) is not None:
        out.extend(_fence_lines("", "toml", "`pyproject.toml`", _body(lines, config), linenos=False))
        previous = config["end_line"]
        skip_blank = False
    for block in example["blocks"]:
        if not skip_blank:
            out.extend(lines[previous : block["line"] - 1])
        previous = block["end_line"]
        skip_blank = block["role"] == "context"
        if not skip_blank:
            out.extend(
                _fence_lines("", block["language"], f"`{block['file']}`" if block["named"] else "", _body(lines, block))
            )
    return out, skip_blank


def _code_block(indent: str, language: str, label: str, body: list[str]) -> list[str]:
    return [*_fence_lines(indent, language, label, body), ""]


# the file name is a paragraph rather than a caption, since sphinx-llm cannot translate captions
def _fence_lines(indent: str, language: str, label: str, body: list[str], *, linenos: bool = True) -> list[str]:
    heading = [f"{indent}{label}", ""] if label else []
    # numbered lines tell a soft-wrapped line from the next one, which matters in the narrow side-by-side columns
    numbering = [f"{indent}:linenos:"] if linenos else []
    return [*heading, f"{indent}```{{code-block}} {language}", *numbering, "", *body, f"{indent}```"]


def _write(path: Path, content: str) -> None:
    # sphinx-llm reruns this configuration in a second build that reads these files, so rewrite only what changed
    if not path.exists() or path.read_text(encoding="utf-8") != content:
        path.write_text(content, encoding="utf-8")


class _PageContext(TypedDict):
    # The keys these handlers read or write; Sphinx's context holds many more.
    rule_sources: dict[str, str]
    body: str
    furo_navigation_tree: str
    theme_source_edit_link: NotRequired[str]
    theme_source_view_link: NotRequired[str]


def setup(app: Sphinx) -> None:
    app.connect("write-started", _draw_diagrams)
    app.connect("doctree-resolved", _add_controls)
    # after Furo's own handler, which builds the navigation tree this one marks
    app.connect("html-page-context", _settle_page, priority=600)
    # after Furo's own handler, which writes the dark highlighting styles into pygments.css when the build finishes
    app.connect("build-finished", _inline_assets, priority=900)


def _draw_diagrams(app: Sphinx, builder: Builder) -> None:
    # The extension starts a browser for each diagram as it writes the page, and with parallel writers several start at
    # once, more than a small container can launch in time. One mermaid-cli run draws every diagram the pages hold
    # before they are written, under the name the extension looks for, so it reuses them and starts no browser; a
    # rebuild draws only diagrams that changed.
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
        command = [*shlex.split(mermaid_cmd), *mermaid_params, "--configFile", mermaid_sequence_config]
        command += ["--input", str(work / "diagrams.md"), "--output", str(work / "drawn.md")]
        run = subprocess.run(command, capture_output=True, text=True, check=False)  # ruff: ignore[subprocess-without-shell-equals-true] # the pinned command above
        if run.returncode:
            msg = f"mermaid-cli could not draw the diagrams:\n{run.stderr}{run.stdout}"
            raise RuntimeError(msg)
        # mermaid-cli numbers the drawings of a Markdown file in the order of its diagrams
        for index, target in enumerate(pending, start=1):
            shutil.move(work / f"drawn-{index}.svg", target)


def _add_controls(app: Sphinx, doctree: nodes.document, docname: str) -> None:
    # Pages are built in their final shape, so no script moves them once they show: rules.js only wires up the
    # controls. They are HTML alone, so the Markdown copies of the pages leave them out.
    del docname
    if app.builder.format != "html":
        return
    raws = list(doctree.findall(nodes.raw))
    # filters from the URL apply before anything below the table exists, so nothing below moves once it shows
    if (table := next((node for node in raws if 'id="rule-empty"' in node.astext()), None)) is not None:
        script = '<script>setupRulesTable(document.querySelector("table.rules"))</script>'
        table.replace_self(nodes.raw("", f"{table.astext()}{script}", format="html"))
    examples = list(doctree.findall(dropdown_main))
    # one pill in the badge row opens every example, or closes them all once each is open; rules.js keeps its label
    badges = next((node for node in raws if '<div class="badges">' in node.astext()), None)
    if examples and badges is not None:
        switch = '<button type="button" class="badge example-switch" aria-expanded="false">Expand all examples</button>'
        badges.replace_self(nodes.raw("", badges.astext().replace("</div>", f"{switch}</div>"), format="html"))
    count = 0
    for example in examples:
        classes = example["classes"]
        if "example-fix" in classes or "example-pair" in classes:
            _add_measures(example)
        kind = next((name for name in _EXAMPLE_TOGGLES if name in classes), None)
        item = next((parent for parent in _ancestors(example) if isinstance(parent, nodes.list_item)), None)
        paragraph = (
            None
            if item is None
            else next((child for child in item.children if isinstance(child, nodes.paragraph)), None)
        )
        if kind is None or paragraph is None:
            continue
        if not example["ids"]:
            count += 1
            example["ids"].append(f"rule-example-{count}")
        icon, label = _EXAMPLE_TOGGLES[kind]
        # a pair's icon is both marks, colored apart by the stylesheet
        marks = "".join(
            f'<span class="mark-{"reported" if mark == "✗" else "not-reported"}">{mark}</span>'
            if kind == "example-pair"
            else f"<span>{mark}</span>"
            for mark in icon
        )
        button = (
            f'<button type="button" class="example-toggle {kind}" title="{label}" aria-label="{label}"'
            f' aria-controls="{example["ids"][0]}" aria-expanded="{str(bool(example.get("opened"))).lower()}">'
            f"{marks}</button>"
        )
        paragraph.append(nodes.raw("", button, format="html"))
        classes.append("example-inline")


def _add_measures(example: nodes.Element) -> None:
    # Before and after sit side by side only while every line fits its half. Each side holds an invisible copy of the
    # longest line of both, built like the snippets around it, so the browser sizes both sides alike and wraps the
    # second below the first once that line no longer fits; no script measures the page.
    sides = [node for node in example.findall(nodes.container) if "example-side" in node["classes"]]
    lines = [
        line for side in sides for block in side.findall(nodes.literal_block) for line in block.astext().splitlines()
    ]
    longest = html.escape(max(lines, key=len, default=""))
    measure = (
        '<div class="highlight-text example-measure" aria-hidden="true"><div class="highlight"><pre>'
        f'<span class="linenos">0</span>{longest}</pre></div></div>'
    )
    for side in sides:
        side.insert(0, nodes.raw("", measure, format="html"))
    # an odd side out keeps to half the width beside an empty one, as the sides above it do
    if len(sides) % 2 and (parent := sides[-1].parent) is not None:
        filler = f'<div class="example-side" aria-hidden="true">{measure}</div>'
        parent.insert(parent.index(sides[-1]) + 1, nodes.raw("", filler, format="html"))


def _ancestors(node: nodes.Element) -> list[nodes.Element]:
    found: list[nodes.Element] = []
    while (node := node.parent) is not None:
        found.append(node)
    return found


def _settle_page(
    app: Sphinx, pagename: str, templatename: str, context: _PageContext, doctree: nodes.document | None
) -> None:
    del templatename, doctree
    _point_generated_pages_at_their_source(pagename, context)
    if "furo_navigation_tree" in context:
        context["furo_navigation_tree"] = _SIDEBAR_LINK.sub(
            partial(_mark_sidebar_link, pagename), context["furo_navigation_tree"]
        )
    if "body" in context:
        diagrams = Path(app.outdir) / app.builder.imagedir
        context["body"] = _DIAGRAM.sub(
            partial(_inline_diagram, diagrams), _NUMBERED_CODE.sub(_split_code_lines, context["body"])
        )


def _point_generated_pages_at_their_source(pagename: str, context: _PageContext) -> None:
    # Generated pages have no file under docs/, so the theme's view and edit icons would point nowhere: a rule page
    # comes from its explanation in the crate, the group pages from this configuration.
    if pagename.startswith("_generated/rules/"):
        source = context["rule_sources"][pagename.rsplit("/", 1)[1]]
    elif pagename.startswith("_generated/"):
        source = "docs/conf.py"
    else:
        return
    context["theme_source_edit_link"] = f"https://github.com/gaborbernat/simplint/edit/main/{source}"
    context["theme_source_view_link"] = f"https://github.com/gaborbernat/simplint/blob/main/{source}"


def _mark_sidebar_link(pagename: str, match: re.Match[str]) -> str:
    # The stylesheet gives each sidebar entry the icon of the page it names. Rule pages lead with their code and take
    # none; Furo links the current page as `#`.
    classes, href = match[1].split(), match[2]
    if "internal" not in classes:
        return match[0]
    path = href.split("#", 1)[0]
    target = pagename if not path else posixpath.normpath(posixpath.join(posixpath.dirname(pagename), path))
    if target.startswith("_generated/rules/"):
        return match[0]
    # a folder's own index page is named after the folder, so it does not take the home page's icon
    *folders, page = target.removesuffix(".html").split("/")
    return f'{match[0]} data-page="{folders[-1] if page == "index" and folders else page}"'


def _inline_diagram(diagrams: Path, match: re.Match[str]) -> str:
    # In the page, rather than behind an object element, a diagram takes its size from its viewBox as the page lays out,
    # and the stylesheet colors its edges and labels for the theme. Each keeps ids of its own, as a page may hold two.
    svg = (diagrams / match["name"]).read_text(encoding="utf-8").replace("my-svg", match["name"].removesuffix(".svg"))
    return f'<div class="mermaid">{svg}</div>'


def _split_code_lines(match: re.Match[str]) -> str:
    # The highlighter writes a numbered snippet as one run of text with the numbers inline, so a wrapped line would
    # continue under the numbers. One block per line lets the stylesheet hang each continuation under its code; each
    # keeps its newline, so copied code keeps its lines.
    if '<span class="linenos">' not in (code := match[2]):
        return match[0]
    lines = code.splitlines(keepends=True)
    # Pygments closes every token at the end of its line, so each line is whole markup on its own
    if unbalanced := [line for line in lines if line.count("<span") != line.count("</span>")]:
        msg = f"a highlighted line leaves a token open: {unbalanced[0]!r}"
        raise ValueError(msg)
    return match[1] + "".join(f'<span class="code-line">{line}</span>' for line in lines) + match[3]


def _inline_assets(app: Sphinx, exception: Exception | None) -> None:
    # Read the Docs answers every request after a round trip of 200 ms or more, so each stylesheet and script a page
    # waits on would add one; each page carries its own instead, and arrives ready to render in a single response.
    if exception is not None or app.builder.format != "html":
        return
    # Furo writes the dark highlighting into pygments.css as the build finishes; inlined before that, every snippet
    # would stay light in the dark theme
    highlighting = (Path(app.outdir) / "_static" / "pygments.css").read_text(encoding="utf-8")
    if 'body[data-theme="dark"] .highlight' not in highlighting:
        msg = "pygments.css has no dark theme yet, so the pages would inline light code blocks for the dark theme"
        raise RuntimeError(msg)
    for page in Path(app.outdir).rglob("*.html"):
        text = page.read_text(encoding="utf-8")

        def inline(match: re.Match[str], page: Path = page) -> str:
            asset = (page.parent / (match["css"] or match["js"])).resolve()
            content = _SOURCE_MAP.sub("", asset.read_text(encoding="utf-8"))
            return f"<style>{content}</style>" if match["css"] else f"<script>{content}</script>"

        page.write_text(_ASSET.sub(inline, text), encoding="utf-8")


# Each example dropdown of a rule page gets an icon at the end of its list item's first paragraph, which toggles it.
_EXAMPLE_TOGGLES: Final[dict[str, tuple[str, str]]] = {
    "example-reported": ("✗", "Show an example it reports"),
    "example-not-reported": ("✓", "Show an example it accepts"),
    "example-pair": ("✗✓", "Show an example it reports beside one it accepts"),
    "example-fix": ("🔧", "Show the fix"),
    "example-configuration": ("⚙", "Show the configuration"),
}


_SIDEBAR_LINK: Final[re.Pattern[str]] = re.compile(r'<a class="([^"]*)" href="([^"]*)"')


_NUMBERED_CODE: Final[re.Pattern[str]] = re.compile(r'(<div class="highlight"><pre>)(.*?)(</pre>)', re.DOTALL)


_DIAGRAM: Final[re.Pattern[str]] = re.compile(
    r'<object data="[^"]*?(?P<name>mermaid-[0-9a-f]+\.svg)" type="image/svg\+xml">.*?</object>', re.DOTALL
)


_ASSET: Final[re.Pattern[str]] = re.compile(
    r'<link rel="stylesheet" type="text/css" href="(?P<css>[^"?]*_static/[^"?]+)(?:\?[^"]*)?" />'
    r'|<script src="(?P<js>[^"?]*_static/[^"?]+)(?:\?[^"]*)?"></script>'
)


_SOURCE_MAP: Final[re.Pattern[str]] = re.compile(r"^\s*(?://|/\*)# sourceMappingURL=.*$", re.MULTILINE)


# every page's template context, where the edit and view links of a rule page find the file it comes from
html_context: Final[dict[str, dict[str, str]]] = {"rule_sources": _generate(Path(__file__).parent)}
