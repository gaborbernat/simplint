"""Configure the documentation build."""

from __future__ import annotations

import html
import json
import os
import re
import sys
import textwrap
import tomllib
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING, Final, NotRequired, TypedDict

if TYPE_CHECKING:
    from docutils import nodes
    from sphinx.application import Sphinx

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
# cppreference answers automated clients with 403 Forbidden while the page loads in a browser; the changelog links each
# entry's pull request and author profile, which the changelog task builds from real pull requests, and checking several
# hundred of them makes GitHub answer 504
linkcheck_ignore: Final[list[str]] = [
    r"https://en\.cppreference\.com/.*",
    r"https://github\.com/gaborbernat/simplint/pull/\d+$",
    r"https://github\.com/[\w-]+$",
]
linkcheck_anchors_ignore_for_url: Final[list[str]] = [
    r"https://github\.com/.*",
    r"https://microsoft\.github\.io/pyright/.*",
]

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
    for folder, current in (
        ("rules", {f"{rule['code']}.md" for rule in rules}),
        ("groups", {f"{family}.rst" for family in groups}),
    ):
        for stale in (page for page in (generated / folder).iterdir() if page.name not in current):
            stale.unlink()
    # each explanation sits next to its rule, in the directory of the rule's family
    root = docs.parent
    return {
        rule["code"]: next(root.glob(f"src/rules/*/{rule['name'].replace('-', '_')}.md")).relative_to(root).as_posix()
        for rule in rules
    }


_FIX_BADGES: Final[dict[str, tuple[str, str]]] = {
    "Always": ("fix-always", "🛠️ Fix always available"),
    "Sometimes": ("fix-sometimes", "🔧 Fix sometimes available"),
    "None": ("fix-none", "No fix"),
}


def _cross_file_badge(root: str) -> str:
    # links to the page that explains what reading other files means for a rule; `root` leads back to the docs root
    return (
        f'<a class="badge cross-file" href="{root}explanation/how-it-works.html#cross-file-rules"'
        ' title="Other files of the run can change what this rule reports">🔗 Cross-file</a>'
    )


def _rules_index(rules: list[_RuleEntry], groups: dict[str, tuple[str, str]]) -> str:
    options = "".join(
        f'<option value="{family}">{html.escape(group)}</option>' for family, (group, _) in groups.items()
    )
    rows: list[str] = []
    for family, (group, _) in groups.items():
        rows.append(f'<tr class="group-row" data-group="{family}"><td colspan="4">{html.escape(group)}</td></tr>')
        for rule in (entry for entry in rules if entry["family"] == family):
            badge_class, badge_text = _FIX_BADGES[rule["fix"]]
            fix = (
                ""
                if rule["fix"] == "None"
                else f'<span class="badge {badge_class}" title="{badge_text}" aria-label="{badge_text}">'
                f"{badge_text.split(' ', 1)[0]}</span>"
            )
            cross_file = _cross_file_badge("") if rule["cross_file"] else ""
            searchable = html.escape(f"{rule['code']} {rule['name']} {rule['summary']}".lower(), quote=True)
            rows.append(
                f'<tr class="rule-row" id="{rule["name"]}" data-group="{family}" data-fix="{rule["fix"]}"'
                f' data-text="{searchable}">'
                f'<td class="rule-code" id="{rule["code"]}">'
                f'<a href="_generated/rules/{rule["code"]}.html">{rule["code"]}</a></td>'
                f'<td class="rule-name">{rule["name"]}{cross_file}</td>'
                f'<td class="rule-summary">{_inline_code(rule["summary"])}</td>'
                f'<td class="rule-fix">{fix}</td></tr>'
            )
    body = "\n".join(rows)
    markup = (
        '<div class="rule-filter">'
        '<input id="rule-search" type="search" placeholder="Filter by code, name or description (press /)"'
        ' aria-label="Filter rules">'
        f'<select id="rule-group" aria-label="Group"><option value="">All groups</option>{options}</select>'
        '<select id="rule-fix" aria-label="Fix availability"><option value="">Any fix</option>'
        '<option value="Always">Fix always available</option><option value="Sometimes">Fix sometimes available</option>'
        '<option value="None">No fix</option></select>'
        '<span id="rule-count" class="rule-count"></span></div>'
        '<table class="rules"><thead><tr><th>Code</th><th class="rule-name">Name</th><th>Checks</th><th>Fix</th>'
        f"</tr></thead><tbody>\n{body}\n</tbody></table>"
        '<p id="rule-empty" class="rule-empty" hidden>No rule matches these filters.</p>'
    )
    # the sidebar shows this hidden tree, so a rule page lists its siblings for switching between rules; it has room
    # for one line per entry, so each group shows its short title there
    groups_tree = "\n".join(f"   {short} <_generated/groups/{family}>" for family, (_, short) in groups.items())
    return f".. raw:: html\n\n{textwrap.indent(markup, '   ')}\n\n.. toctree::\n   :hidden:\n\n{groups_tree}\n"


def _group_page(group: str, rules: list[_RuleEntry]) -> str:
    entries = "\n".join(
        f"   {rule['code']} · {rule['name']} <../rules/{rule['code']}>" for rule in rules if rule["group"] == group
    )
    return f"{group}\n{'=' * len(group)}\n\n.. toctree::\n   :titlesonly:\n\n{entries}\n"


def _inline_code(text: str) -> str:
    return re.sub(r"`([^`]+)`", r"<code>\1</code>", html.escape(text))


def _rule_page(rule: _RuleEntry) -> str:
    # rule pages stay Markdown, rendered by MyST, so links on inline code and fenced diagrams survive as written
    if "error" in (examples := rule["examples"]):
        msg = f"{rule['code']}: {examples['error']}"
        raise ValueError(msg)
    title = f"{rule['name']} ({rule['code']})"
    badge_class, badge_text = _FIX_BADGES[rule["fix"]]
    badges = (
        f'<div class="badges"><span class="badge code">{rule["code"]}</span>'
        f'<span class="badge {badge_class}">{badge_text}</span><span class="badge">{html.escape(rule["group"])}</span>'
        f"{_cross_file_badge('../../') if rule['cross_file'] else ''}"
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
    active = False
    index = start
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
            out.extend(_code_block(indent, "python", label, _body(lines, block)))
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
            out.extend(_code_block(indent, "python", file, body))
            continue
        side = "after" if block["role"] == "fixed" else "before"
        out.append(f"{indent}:::{{container}} example-side")
        out.extend(["", *_code_block(indent, "python", _caption(block, side), body)])
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
            out.extend(_fence_lines("", "python", f"`{block['file']}`" if block["named"] else "", _body(lines, block)))
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
    # The keys this handler reads or writes; Sphinx's context holds many more.
    rule_sources: dict[str, str]
    theme_source_edit_link: NotRequired[str]
    theme_source_view_link: NotRequired[str]


def setup(app: Sphinx) -> None:
    app.connect("html-page-context", _point_generated_pages_at_their_source)


def _point_generated_pages_at_their_source(
    app: Sphinx, pagename: str, templatename: str, context: _PageContext, doctree: nodes.document | None
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
