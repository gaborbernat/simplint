"""Write the rule and group pages of the documentation from the rule list, before Sphinx reads the sources."""

from __future__ import annotations

import html
import re
import textwrap
from typing import TYPE_CHECKING, Final

from rules_data import Block, Example, Fence, RuleEntry, rule_entries

if TYPE_CHECKING:
    from pathlib import Path


def generate(docs: Path) -> dict[str, str]:
    # the rule list is derived from the sources the binary is built from, and `tests/docs_data.rs` holds it to the
    # binary's own listing, so the pages and `simplint rule` cannot drift
    rules = rule_entries()
    # groups are keyed by family, the directory of their rules, since titles have spaces; the binary lists rules by
    # code, so each group appears where its first rule does
    groups = {rule["family"]: (rule["group"], rule["short_group"]) for rule in rules}
    rules_dir, groups_dir = docs / "rules", docs / "groups"
    rules_dir.mkdir(exist_ok=True)
    groups_dir.mkdir(exist_ok=True)
    write_if_changed(docs / "_derived" / "rules-table.inc", _rules_index(rules, groups))
    for family, (title, _) in groups.items():
        write_if_changed(groups_dir / f"{family}.rst", _group_page(title, rules))
    for rule in rules:
        write_if_changed(rules_dir / f"{rule['code']}.md", _rule_page(rule))
    remove_stale(rules_dir, {f"{rule['code']}.md" for rule in rules})
    remove_stale(groups_dir, {f"{family}.rst" for family in groups})
    root = docs.parent
    explanations: dict[str, str] = {}
    for rule in rules:
        pattern = f"src/rules/*/{rule['name'].replace('-', '_')}.md"
        if (found := next(root.glob(pattern), None)) is None:
            msg = f"no explanation for {rule['code']}: expected a file matching {pattern}"
            raise FileNotFoundError(msg)
        explanations[rule["code"]] = found.relative_to(root).as_posix()
    return explanations


def write_if_changed(path: Path, content: str) -> None:
    # sphinx-llm reruns this configuration in a second build that reads these files, so rewrite only what changed
    if not path.exists() or path.read_text(encoding="utf-8") != content:
        path.write_text(content, encoding="utf-8")


def _rules_index(rules: list[RuleEntry], groups: dict[str, tuple[str, str]]) -> str:
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
    groups_tree = "\n".join(f"   {short} <groups/{family}>" for family, (_, short) in groups.items())
    return f".. raw:: html\n\n{textwrap.indent(markup, '   ')}\n\n.. toctree::\n   :hidden:\n\n{groups_tree}\n"


def _rule_row(rule: RuleEntry) -> str:
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
        f'<a href="rules/{rule["code"]}.html">{rule["code"]}</a></td>'
        f'<td class="rule-name">{rule["name"]}</td>'
        f'<td class="rule-summary">{summary}</td>'
        f'<td class="rule-fix">{fix}</td>'
        f'<td class="rule-scope">{"✓" if rule["cross_file"] else "✗"}</td></tr>'
    )


def _group_page(group: str, rules: list[RuleEntry]) -> str:
    entries = "\n".join(
        f"   {rule['code']} · {rule['name']} <../rules/{rule['code']}>" for rule in rules if rule["group"] == group
    )
    return f"{group}\n{'=' * len(group)}\n\n.. toctree::\n   :titlesonly:\n\n{entries}\n"


def _rule_page(rule: RuleEntry) -> str:
    # rule pages stay Markdown, rendered by MyST, so links on inline code and fenced diagrams survive as written
    if "error" in (examples := rule["examples"]):
        msg = f"{rule['code']}: {examples['error']}"
        raise ValueError(msg)
    title = f"{rule['name']} ({rule['code']})"
    badge_class, badge_text = _FIX_BADGES[rule["fix"]]
    cross_file = (
        '<a class="badge cross-file" href="../explanation/how-it-works.html#cross-file-rules"'
        ' title="Other files of the run can change what this rule reports">🔗 Cross-file</a>'
        if rule["cross_file"]
        else ""
    )
    badges = (
        f'<div class="badges"><span class="badge code">{rule["code"]}</span>'
        f'<span class="badge {badge_class}">{badge_text}</span><span class="badge">{html.escape(rule["group"])}</span>'
        f"{cross_file}"
        '<a class="badge" href="../rules.html">All rules</a></div>'
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


def _section_body(heading: str, lines: list[str], start: int, stop: int, by_line: dict[int, Example]) -> list[str]:
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


def _examples(lines: list[str], start: int, stop: int, by_line: dict[int, Example], *, settings: bool) -> str:
    # Examples with no prose between them form a run, rendered together so a reported one can sit beside its accepted
    # one. Only the Active on cases pair a reported example with its accepted one.
    out: list[str] = []
    run: list[Example] = []
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


def _gap_lines(lines: list[str], example: Example) -> list[str]:
    fences = [*([config] if (config := _own_config(example)) is not None else []), *example["blocks"]]
    gaps: list[str] = []
    previous = example["line"] - 1
    for fence in fences:
        gaps.extend(lines[previous : fence["line"] - 1])
        previous = fence["end_line"]
    return gaps


def _own_config(example: Example) -> Fence | None:
    # an Example section config holds for the examples after it, but only the example it opens shows it
    config = example["config"]
    return config if config is not None and config["line"] == example["line"] else None


def _render(run: list[Example], lines: list[str], *, settings: bool, pair: bool) -> list[str]:
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
def _pairs(reported: Example, accepted: Example) -> bool:
    shown = ({block["role"] for block in example["blocks"]} - {"context"} for example in (reported, accepted))
    return list(shown) == [{"report"}, {"ok"}] and _config_source(reported) == _config_source(accepted)


def _config_source(example: Example) -> str | None:
    return None if (config := example["config"]) is None else config["source"]


def _pair(reported: Example, accepted: Example, lines: list[str]) -> list[str]:
    indent = _indent(lines, reported)
    out = [
        f"{indent}::::{{dropdown}} Example: reported and not",
        f"{indent}:class-container: example example-pair",
        "",
    ]
    if (config := reported["config"]) is not None:
        out.extend(_code_block(indent, "toml", "`pyproject.toml`", _body(lines, config)))
    for example, side in ((reported, "reported"), (accepted, "not reported")):
        out.extend([f"{indent}:::{{container}} example-side", ""])
        for block in example["blocks"]:
            label = f"`{block['file']}`" if block["role"] == "context" else _caption(block, side)
            out.extend(_code_block(indent, block["language"], label, _body(lines, block)))
        out.extend([f"{indent}:::", ""])
    return [*out, f"{indent}::::", ""]


def _indent(lines: list[str], example: Example) -> str:
    # the fence indentation, which the dropdown keeps so it sits inside the list item
    line = lines[example["line"] - 1]
    return line[: len(line) - len(line.lstrip())]


def _code_block(indent: str, language: str, label: str, body: list[str]) -> list[str]:
    return [*_fence_lines(indent, language, label, body), ""]


# the file name is a paragraph rather than a caption, since sphinx-llm cannot translate captions
def _fence_lines(indent: str, language: str, label: str, body: list[str], *, linenos: bool = True) -> list[str]:
    heading = [f"{indent}{label}", ""] if label else []
    # numbered lines tell a soft-wrapped line from the next one, which matters in the narrow side-by-side columns
    numbering = [f"{indent}:linenos:"] if linenos else []
    return [*heading, f"{indent}```{{code-block}} {language}", *numbering, "", *body, f"{indent}```"]


def _body(lines: list[str], fence: Fence) -> list[str]:
    return lines[fence["line"] : fence["end_line"] - 1]


def _caption(block: Block, side: str) -> str:
    return f"`{block['file']}` {side}" if block["named"] else side.capitalize()


def _dropdown(example: Example, lines: list[str], *, settings: bool) -> list[str]:
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
def _numbered(lines: list[str], start: int, stop: int, by_line: dict[int, Example]) -> str:
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
def _cards(lines: list[str], example: Example, *, skip_blank: bool) -> tuple[list[str], bool]:
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


def remove_stale(folder: Path, current: set[str]) -> None:
    # a page whose rule or group is gone would still build, under a name that no longer exists
    for stale in (page for page in folder.iterdir() if page.name not in current):
        stale.unlink()


_FIX_BADGES: Final[dict[str, tuple[str, str]]] = {
    "Always": ("fix-always", "🛠️ Fix always available"),
    "Sometimes": ("fix-sometimes", "🔧 Fix sometimes available"),
    "None": ("fix-none", "No fix"),
}
