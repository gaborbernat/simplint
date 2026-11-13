"""Settle each built HTML page: the rule page controls, the sidebar marks, the numbered code and the inlined assets."""

from __future__ import annotations

import html
import posixpath
import re
from functools import partial
from pathlib import Path
from types import MappingProxyType
from typing import TYPE_CHECKING, Final, NotRequired, TypedDict

from docutils import nodes
from sphinx_design.dropdown import dropdown_main

if TYPE_CHECKING:
    from sphinx.application import Sphinx


class _PageContext(TypedDict):
    # The keys these handlers read or write; Sphinx's context holds many more.
    rule_sources: dict[str, str]
    body: str
    furo_navigation_tree: str
    theme_source_edit_link: NotRequired[str]
    theme_source_view_link: NotRequired[str]


def setup(app: Sphinx) -> dict[str, bool]:
    app.connect("doctree-resolved", _add_controls)
    # after Furo's own handler, which builds the navigation tree this one marks
    app.connect("html-page-context", _settle_page, priority=600)
    # after Furo's own handler, which writes the dark highlighting styles into pygments.css when the build finishes
    app.connect("build-finished", _inline_assets, priority=900)
    return {"parallel_read_safe": True, "parallel_write_safe": True}


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
    badges = next((node for node in raws if '<div class="badges">' in node.astext()), None)
    if badges is not None and any("example-static" not in example["classes"] for example in examples):
        switch = '<button type="button" class="badge example-switch" aria-expanded="false">Expand all examples</button>'
        badges.replace_self(nodes.raw("", badges.astext().replace("</div>", f"{switch}</div>"), format="html"))
    _add_example_toggles(examples)


def _add_example_toggles(examples: list[dropdown_main]) -> None:
    count = 0
    for example in examples:
        classes = example["classes"]
        if "example-fix" in classes or "example-pair" in classes:
            _add_measures(example)
        if "example-static" in classes:
            continue
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
    lines: list[str] = []
    for side in sides:
        for block in side.findall(nodes.literal_block):
            lines.extend(block.astext().splitlines())
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
    del app, templatename, doctree
    _point_generated_pages_at_their_source(pagename, context)
    if "furo_navigation_tree" in context:
        context["furo_navigation_tree"] = _SIDEBAR_LINK.sub(
            partial(_mark_sidebar_link, pagename), context["furo_navigation_tree"]
        )
    if "body" in context:
        context["body"] = _NUMBERED_CODE.sub(_split_code_lines, context["body"])


def _point_generated_pages_at_their_source(pagename: str, context: _PageContext) -> None:
    # Generated pages have no file under docs/, so the theme's view and edit icons would point nowhere: a rule page
    # comes from its explanation in the crate, the group pages from the generator.
    if pagename.startswith("rules/"):
        source = context["rule_sources"][pagename.rsplit("/", 1)[1]]
    elif pagename.startswith("groups/"):
        source = "docs/_ext/rule_pages.py"
    else:
        return
    context["theme_source_edit_link"] = f"https://github.com/gaborbernat/simplint/edit/main/{source}"
    context["theme_source_view_link"] = f"https://github.com/gaborbernat/simplint/blob/main/{source}"


def _mark_sidebar_link(pagename: str, match: re.Match[str]) -> str:
    # Rule pages lead with their code and take no icon; Furo links the current page as `#`.
    classes, href = match["classes"].split(), match["href"]
    if "internal" not in classes:
        return match[0]
    path = href.split("#", 1)[0]
    target = pagename if not path else posixpath.normpath(posixpath.join(posixpath.dirname(pagename), path))
    if target.startswith("rules/"):
        return match[0]
    # a folder's own index page is named after the folder, so it does not take the home page's icon
    *folders, page = target.removesuffix(".html").split("/")
    return f'{match[0]} data-page="{folders[-1] if page == "index" and folders else page}"'


def _split_code_lines(match: re.Match[str]) -> str:
    # The highlighter writes a numbered snippet as one run of text with the numbers inline, so a wrapped line would
    # continue under the numbers. One block per line lets the stylesheet hang each continuation under its code; each
    # keeps its newline, so copied code keeps its lines.
    if '<span class="linenos">' not in (code := match["code"]):
        return match[0]
    lines = code.splitlines(keepends=True)
    # Pygments closes every token at the end of its line, so each line is whole markup on its own
    if unbalanced := [line for line in lines if line.count("<span") != line.count("</span>")]:
        msg = f"a highlighted line leaves a token open: {unbalanced[0]!r}"
        raise ValueError(msg)
    return match["open"] + "".join(f'<span class="code-line">{line}</span>' for line in lines) + match["close"]


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


_EXAMPLE_TOGGLES: Final[MappingProxyType[str, tuple[str, str]]] = MappingProxyType({
    "example-reported": ("✗", "Show an example it reports"),
    "example-not-reported": ("✓", "Show an example it accepts"),
    "example-pair": ("✗✓", "Show an example it reports beside one it accepts"),
    "example-fix": ("🔧", "Show the fix"),
    "example-configuration": ("⚙", "Show the configuration"),
})


_SIDEBAR_LINK: Final[re.Pattern[str]] = re.compile(
    r"""
    <a[ ]class="       # opening link and class attribute
    (?P<classes>[^"]*)  # class names
    "[ ]href="         # href attribute
    (?P<href>[^"]*)"    # target and closing quote
    """,
    re.VERBOSE,
)


_NUMBERED_CODE: Final[re.Pattern[str]] = re.compile(
    r"""
    (?P<open><div[ ]class="highlight"><pre>)  # code block opening
    (?P<code>.*?)                           # highlighted code
    (?P<close></pre>)                       # code block closing
    """,
    re.DOTALL | re.VERBOSE,
)
_ASSET: Final[re.Pattern[str]] = re.compile(
    r"""
    <link[ ]rel="stylesheet"[ ]type="text/css"[ ]href="  # stylesheet link
    (?P<css>[^"?]*_static/[^"?]+)                     # stylesheet path
    (?:\?[^"]*)?"[ ]/>                               # optional query and closing tag
    |
    <script[ ]src="                                   # script tag
    (?P<js>[^"?]*_static/[^"?]+)                      # script path
    (?:\?[^"]*)?"></script>                           # optional query and closing tag
    """,
    re.VERBOSE,
)


_SOURCE_MAP: Final[re.Pattern[str]] = re.compile(r"^\s*(?://|/\*)# sourceMappingURL=.*$", re.MULTILINE)
