from __future__ import annotations

import json
import re
from typing import TYPE_CHECKING

import pytest

from docs.rules_data import main, parse, rule_entries

if TYPE_CHECKING:
    from pathlib import Path


@pytest.mark.parametrize(
    ("explanation", "reason"),
    [
        pytest.param("```python report\nX = 1\n", "line 1: unclosed code block", id="unclosed_block"),
        pytest.param("```python loud\nX = 1\n```\n", "line 1: unknown role `loud`", id="unknown_role"),
        pytest.param("```python report size=2\n```\n", "line 1: unknown attribute `size=2`", id="unknown_attribute"),
        pytest.param(
            "```python report file=a.py file=b.py\n```\n",
            "line 1: repeated attribute `file`",
            id="repeated_file",
        ),
        pytest.param(
            "```python report executable executable\n```\n",
            "line 1: repeated attribute `executable`",
            id="repeated_executable",
        ),
        pytest.param(
            "```python report executable=yes\n```\n",
            "line 1: unknown attribute `executable=yes`",
            id="executable_with_value",
        ),
        pytest.param(
            "```python ok select=\n```\n",
            "line 1: `select=` takes comma-separated rule codes, such as `SMP009,SMP101`",
            id="empty_select",
        ),
        pytest.param(
            "```python ok select=SMP009,SMP-101\n```\n",
            "line 1: `select=` takes comma-separated rule codes, such as `SMP009,SMP101`",
            id="select_with_punctuation",
        ),
        pytest.param(
            "```python ok select=SMP009,\n```\n",
            "line 1: `select=` takes comma-separated rule codes, such as `SMP009,SMP101`",
            id="select_with_empty_code",
        ),
        pytest.param(
            "```python fixed\nX = 1\n```\n",
            "line 1: `fixed` must directly follow the `report` block of its file",
            id="fixed_first",
        ),
        pytest.param(
            "```python report\nX = 1\n```\n\n```python fixed file=a.py\nX = 1\n```\n",
            "line 5: `fixed` must directly follow the `report` block of its file",
            id="fixed_of_other_file",
        ),
        pytest.param(
            "```toml config\n[tool]\n```\n",
            "example 1: a config block has no python block after it",
            id="config_alone",
        ),
        pytest.param(
            "```rst ok file=a.rst\nA\n```\n",
            "line 1: a `rst` block is a `context` block with a file",
            id="markup_not_context",
        ),
        pytest.param(
            "```rst context\nA\n```\n",
            "line 1: a `rst` block is a `context` block with a file",
            id="markup_without_file",
        ),
    ],
)
def test_parse_rejects_malformed_examples(explanation: str, reason: str) -> None:
    with pytest.raises(ValueError, match=re.escape(reason)):
        parse(explanation)


def test_parse_joins_a_fixed_block_to_the_report_of_its_file() -> None:
    examples = parse("```python report\nX = 1\n```\n\n```python fixed\nX = 2\n```")

    assert [[block["role"] for block in example["blocks"]] for example in examples] == [["report", "fixed"]]


@pytest.mark.usefixtures("registry")
def test_rule_entries_reads_the_registry_row(tmp_path: Path) -> None:
    (tmp_path / "src" / "rules" / "api").mkdir(parents=True)
    (tmp_path / "src" / "rules" / "api" / "all_format.md").write_text("Flags it.\n", encoding="utf-8")

    assert rule_entries(tmp_path) == [
        {
            "code": "SMP405",
            "name": "all-format",
            "summary": "`__all__` format",
            "fix": "Always",
            "explanation": "Flags it.\n",
            "examples": {"parsed": []},
            "family": "api",
            "group": "Public API",
            "short_group": "Public API",
            "cross_file": False,
        }
    ]


@pytest.mark.usefixtures("registry")
def test_rule_entries_lists_a_page_that_does_not_parse_as_an_error(tmp_path: Path) -> None:
    (tmp_path / "src" / "rules" / "api").mkdir(parents=True)
    (tmp_path / "src" / "rules" / "api" / "all_format.md").write_text("```python loud\nX = 1\n```\n", encoding="utf-8")

    assert rule_entries(tmp_path)[0]["examples"] == {"error": "line 1: unknown role `loud`"}


@pytest.mark.usefixtures("registry")
def test_rule_entries_keeps_a_crlf_page_as_written(tmp_path: Path) -> None:
    (tmp_path / "src" / "rules" / "api").mkdir(parents=True)
    page = "Flags it.\r\n\r\n```python report\r\nX = 1\r\n```\r\n"
    (tmp_path / "src" / "rules" / "api" / "all_format.md").write_bytes(page.encode("utf-8"))

    entry = rule_entries(tmp_path)[0]

    assert (entry["explanation"], entry["examples"]["parsed"][0]["blocks"][0]["source"]) == (page, "X = 1\n")


@pytest.mark.parametrize(
    ("old", "new", "reason"),
    [
        pytest.param("rules! {", "rule! {", "registry has no block matching", id="no_rule_table"),
        pytest.param("pub const GROUPS", "pub const GROUP", "registry has no block matching", id="no_groups"),
        pytest.param("Always, api", "Usually, api", "unrecognized registry line", id="rule_line"),
        pytest.param(
            '("api", "Public API", "Public API"),',
            '("api", "Public API"),',
            "unrecognized registry line",
            id="group_line",
        ),
        pytest.param(
            "`__all__` format",
            r"a \\d b",
            "unrecognized registry line",
            id="backslash_escape",
        ),
    ],
)
def test_rule_entries_rejects_a_registry_it_cannot_read(
    tmp_path: Path, registry: Path, old: str, new: str, reason: str
) -> None:
    registry.write_text(registry.read_text(encoding="utf-8").replace(old, new), encoding="utf-8")

    with pytest.raises(ValueError, match=reason):
        rule_entries(tmp_path)


def test_main_prints_the_rule_entries_as_json(capsys: pytest.CaptureFixture[str]) -> None:
    main()

    assert json.loads(capsys.readouterr().out) == rule_entries()
