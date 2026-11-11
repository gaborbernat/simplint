"""
Derive the rule list `simplint rules --output-format json` prints from the sources the binary is built from.

The rule pages and the Python tasks read this list, so they run without compiling Rust. `tests/docs_data.rs` compares
the derivation with the binary's output, field by field.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Final, TypedDict

_ROOT: Final[Path] = Path(__file__).resolve().parents[1]
_ROLES: Final[frozenset[str]] = frozenset({"report", "ok", "context", "fixed"})
_TABLE: Final[re.Pattern[str]] = re.compile(r"^rules! \{\n(.*?)^\}\n", re.MULTILINE | re.DOTALL)
_GROUPS: Final[re.Pattern[str]] = re.compile(r"^pub const GROUPS: .*? = \[\n(.*?)^\];\n", re.MULTILINE | re.DOTALL)
_ROW: Final[re.Pattern[str]] = re.compile(
    r'\w+ => \("(?P<code>SMP\d{3})", "(?P<name>[^"\\]+)", "(?P<summary>[^"\\]+)", '
    r"(?P<fix>Always|Sometimes|None), (?P<family>\w+)::(?P<check>\w+) \{[^}]*\}"
    r"(?P<attributes>(?: (?:since \w+|reads \[[^\]]*\]|skips \[[^\]]*\]))*)\),"
)
_GROUP: Final[re.Pattern[str]] = re.compile(r'\("\w+", "(?P<name>[^"\\]+)", "(?P<short>[^"\\]+)"\),')


class Fence(TypedDict):
    line: int
    end_line: int
    source: str


class Block(Fence):
    role: str
    language: str
    file: str
    named: bool


class Example(TypedDict):
    section: str | None
    line: int
    end_line: int
    config: Fence | None
    blocks: list[Block]


class Examples(TypedDict, total=False):
    parsed: list[Example]
    error: str


class RuleEntry(TypedDict):
    code: str
    name: str
    summary: str
    fix: str
    explanation: str
    examples: Examples
    family: str
    group: str
    short_group: str
    cross_file: bool


def rule_entries(root: Path = _ROOT) -> list[RuleEntry]:
    registry = (root / "src" / "lint" / "registry.rs").read_text(encoding="utf-8")
    groups = [_group(line) for line in _registry_lines(registry, _GROUPS)]
    return [_rule(root, line, groups) for line in _registry_lines(registry, _TABLE)]


def parse(explanation: str) -> list[Example]:
    return _Parser(explanation).parse()


def main() -> None:
    sys.stdout.write(json.dumps(rule_entries(), indent=2) + "\n")


class _Parser:
    def __init__(self, explanation: str) -> None:
        self._lines = _split_lines(explanation)
        self._examples: list[Example] = []
        self._open: Example | None = None
        self._section: str | None = None
        self._section_config: Fence | None = None

    def parse(self) -> list[Example]:
        index = 0
        while index < len(self._lines):
            if self._lines[index].lstrip().startswith("```"):
                index = self._fence(index)
            else:
                self._prose(self._lines[index])
                index += 1
        for position, example in enumerate(self._examples, start=1):
            if not example["blocks"]:
                msg = f"example {position}: a config block has no python block after it"
                raise ValueError(msg)
        return self._examples

    def _prose(self, line: str) -> None:
        if line.startswith("## "):
            self._section = line.removeprefix("## ")
            self._section_config = None
        if line.strip():
            self._open = None

    def _fence(self, start: int) -> int:
        fence = _read_fence(self._lines, start)
        match self._lines[start].lstrip().removeprefix("```").split():
            case ["toml", "config"]:
                self._config(fence)
            case [("python" | "rst" | "markdown") as language, role, *attributes]:
                self._block(fence, language, role, attributes)
            case _:
                self._open = None
        return fence["end_line"]

    def _config(self, fence: Fence) -> None:
        if self._section == "Example":
            self._section_config = fence
        example = Example(
            section=self._section, line=fence["line"], end_line=fence["end_line"], config=fence, blocks=[]
        )
        self._examples.append(example)
        self._open = example

    def _block(self, fence: Fence, language: str, role: str, attributes: list[str]) -> None:
        number = fence["line"]
        if role not in _ROLES:
            msg = f"line {number}: unknown role `{role}`"
            raise ValueError(msg)
        if language != "python" and (role != "context" or not attributes):
            msg = f"line {number}: a `{language}` block is a `context` block with a file"
            raise ValueError(msg)
        file = _file(attributes, number)
        block = Block(
            role=role,
            language=language,
            file="example.py" if file is None else file,
            named=file is not None,
            line=number,
            end_line=fence["end_line"],
            source=fence["source"],
        )
        example = self._add(block)
        example["end_line"] = fence["end_line"]
        example["config"] = example["config"] or self._section_config
        self._open = example

    def _add(self, block: Block) -> Example:
        if self._open is not None and _joins(self._open, block):
            self._open["blocks"].append(block)
            return self._open
        if block["role"] == "fixed":
            msg = f"line {block['line']}: `fixed` must directly follow the `report` block of its file"
            raise ValueError(msg)
        example = Example(
            section=self._section, line=block["line"], end_line=block["end_line"], config=None, blocks=[block]
        )
        self._examples.append(example)
        return example


def _registry_lines(registry: str, pattern: re.Pattern[str]) -> list[str]:
    match = pattern.search(registry)
    if match is None:
        msg = f"registry has no block matching {pattern.pattern}"
        raise ValueError(msg)
    return [line.strip() for line in match[1].split("\n") if line.strip()]


def _match(pattern: re.Pattern[str], line: str) -> re.Match[str]:
    match = pattern.fullmatch(line)
    if match is None:
        msg = f"unrecognized registry line: {line}"
        raise ValueError(msg)
    return match


def _group(line: str) -> tuple[str, str]:
    match = _match(_GROUP, line)
    return match["name"], match["short"]


def _rule(root: Path, line: str, groups: list[tuple[str, str]]) -> RuleEntry:
    row = _match(_ROW, line)
    family, check = row["family"], row["check"]
    explanation = (root / "src" / "rules" / family / f"{check}.md").read_bytes().decode("utf-8")
    group, short_group = groups[int(row["code"][3])]
    reads = re.search(r"reads \[([^\]]*)\]", row["attributes"])
    return {
        "code": row["code"],
        "name": row["name"],
        "summary": row["summary"],
        "fix": row["fix"],
        "explanation": explanation,
        "examples": _examples(explanation),
        "family": family,
        "group": group,
        "short_group": short_group,
        "cross_file": reads is not None and "cross_file" in reads[1].replace(" ", "").split(","),
    }


def _examples(explanation: str) -> Examples:
    try:
        return {"parsed": parse(explanation)}
    except ValueError as error:
        return {"error": str(error)}


def _split_lines(text: str) -> list[str]:
    *terminated, rest = text.split("\n")
    lines = [line.removesuffix("\r") for line in terminated]
    if rest:
        lines.append(rest)
    return lines


def _read_fence(lines: list[str], start: int) -> Fence:
    indent = len(lines[start]) - len(lines[start].lstrip())
    source: list[str] = []
    index = start + 1
    while index < len(lines) and lines[index].strip() != "```":
        body = lines[index]
        leading = len(body) - len(body.lstrip(" "))
        source.append(body[min(leading, indent) :] + "\n")
        index += 1
    if index == len(lines):
        msg = f"line {start + 1}: unclosed code block"
        raise ValueError(msg)
    return Fence(line=start + 1, end_line=index + 1, source="".join(source))


def _file(attributes: list[str], number: int) -> str | None:
    match attributes:
        case []:
            return None
        case [attribute]:
            if not attribute.startswith("file="):
                msg = f"line {number}: unknown attribute `{attribute}`"
                raise ValueError(msg)
            return attribute.removeprefix("file=")
        case _:
            msg = f"line {number}: more than one attribute"
            raise ValueError(msg)


def _joins(example: Example, block: Block) -> bool:
    if not example["blocks"]:
        return block["role"] != "fixed"
    previous = example["blocks"][-1]
    if block["role"] == "fixed":
        return previous["role"] == "report" and previous["file"] == block["file"]
    return previous["named"] and block["named"]


__all__ = [
    "Block",
    "Example",
    "Examples",
    "Fence",
    "RuleEntry",
    "main",
    "parse",
    "rule_entries",
]

if __name__ == "__main__":
    main()
