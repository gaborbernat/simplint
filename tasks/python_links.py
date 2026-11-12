"""Link the standard-library names the rule pages mention to the Python documentation, by its intersphinx inventory."""

from __future__ import annotations

import argparse
import re
import sys
import urllib.request
from pathlib import Path
from typing import TYPE_CHECKING, Final

from sphinx.util.inventory import InventoryFile

if TYPE_CHECKING:
    from collections.abc import Sequence

_DOCS: Final[str] = "https://docs.python.org/3"
_LINK: Final[re.Pattern[str]] = re.compile(r"\]\((?P<uri>https://docs\.python\.org/3/[^)\s]+)\)")
# Each code span in turn, with the brackets of a link around it, so a closing backtick never opens a span.
_SPAN: Final[re.Pattern[str]] = re.compile(
    r"""
    (?P<open>\[)?        # the bracket opening a link around the span
    `(?P<code>[^`]+)`    # the code span
    (?P<close>\])?       # the bracket closing that link
    """,
    re.VERBOSE,
)
# A name, optionally called: `contextlib.chdir`, `os.chdir(target)`, `TypeError`.
_NAME: Final[re.Pattern[str]] = re.compile(
    r"""
    (?P<name>[A-Za-z_][\w.]*)  # a dotted name
    (?P<call>\(.*\))?          # its arguments, when it is called
    """,
    re.VERBOSE,
)


class Inventory:
    def __init__(self, content: bytes) -> None:
        data = InventoryFile.loads(content, uri=_DOCS).data
        self.uris: set[str] = {item.uri for role in data.values() for item in role.values()}
        # The first role listed wins, so an exception or class is not linked as the function of the same name.
        self.names: dict[str, tuple[str, str]] = {}
        linked_roles: tuple[str, ...] = (
            "py:exception",
            "py:class",
            "py:function",
            "py:method",
            "py:attribute",
            "py:property",
            "py:data",
            "py:module",
        )
        for role in reversed(linked_roles):
            self.names.update({name: (role, item.uri) for name, item in data.get(role, {}).items()})
        # A page that moved keeps its file name and anchors, as `library/functions.html` did into `builtins/`.
        self.moved = {_file_and_anchor(uri): uri for uri in self.uris}
        self.pages = {uri.partition("#")[0] for uri in self.uris}

    def canonical(self, uri: str) -> str | None:
        # The inventory's address of a link, or None when the link names nothing the inventory holds. A section heading
        # has no entry, so a link to a known page keeps its anchor.
        if uri in self.uris:
            return uri
        if (moved := self.moved.get(_file_and_anchor(uri))) is not None:
            return moved
        return uri if uri.partition("#")[0] in self.pages else None

    def target(self, name: str, *, called: bool) -> str | None:
        if (found := self.names.get(name)) is None:
            return None
        role, uri = found
        # An undotted name is often a keyword or a project's own name, such as the `type` statement or a `set`
        # attribute, so only an exception or a builtin written as a call is surely the builtin.
        return uri if "." in name or role == "py:exception" or (role == "py:function" and called) else None


def _file_and_anchor(uri: str) -> tuple[str, str]:
    page, _, anchor = uri.partition("#")
    return page.rpartition("/")[2], anchor


def relink(text: str, inventory: Inventory) -> tuple[str, list[str]]:
    # Move each link to its address in the inventory and link the first mention of each name it holds.
    #
    # :returns: the new text and the links the inventory does not hold
    unknown: list[str] = []

    def move(match: re.Match[str]) -> str:
        if (uri := inventory.canonical(match["uri"])) is None:
            unknown.append(match["uri"])
            return match[0]
        return f"]({uri})"

    lines = text.split("\n")
    linked = {inventory.canonical(match["uri"]) for match in _LINK.finditer(text)}
    fenced = False
    for index, line in enumerate(lines):
        if line.lstrip().startswith("```"):
            fenced = not fenced
            continue
        if fenced or line.startswith("#"):
            continue
        moved = _LINK.sub(move, line)

        def mention(match: re.Match[str]) -> str:
            if match["open"] or match["close"] or not (name := _NAME.fullmatch(match["code"])):
                return match[0]
            uri = inventory.target(name["name"], called=name["call"] is not None)
            if uri is None or uri in linked:
                return match[0]
            linked.add(uri)
            return f"[`{match['code']}`]({uri})"

        lines[index] = _SPAN.sub(mention, moved)
    return "\n".join(lines), unknown


def main(argv: Sequence[str] | None = None) -> int:
    # Check, or with ``--fix`` rewrite, the Python documentation links of the given Markdown files.
    parser = argparse.ArgumentParser(description="Link standard-library names to the Python documentation")
    parser.add_argument("paths", nargs="+", type=Path)
    parser.add_argument("--fix", action="store_true", help="rewrite the files instead of reporting them")
    args = parser.parse_args(argv)
    with urllib.request.urlopen(f"{_DOCS}/objects.inv") as response:
        inventory = Inventory(response.read())
    problems: list[str] = []
    for path in sorted(file for root in args.paths for file in (root.rglob("*.md") if root.is_dir() else [root])):
        text = path.read_text(encoding="utf-8")
        relinked, unknown = relink(text, inventory)
        problems.extend(f"{path}: {uri} is not in the Python inventory" for uri in unknown)
        if relinked == text:
            continue
        if args.fix:
            path.write_text(relinked, encoding="utf-8")
        else:
            problems.append(f"{path}: Python documentation links are missing or stale; run with --fix")
    print("\n".join(problems), file=sys.stderr)
    return int(bool(problems))


if __name__ == "__main__":
    raise SystemExit(main())


__all__ = [
    "Inventory",
    "main",
    "relink",
]
