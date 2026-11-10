from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from tasks.check_changelog_links import main

if TYPE_CHECKING:
    from pathlib import Path


@pytest.mark.parametrize(
    ("line", "expected"),
    [
        pytest.param(
            "[`module-variable-prefix`](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP404.html)",
            1,
            id="latest_rule_link",
        ),
        pytest.param(
            "[rule](http://simplint.readthedocs.io/en/0.10.0/_generated/rules/SMP101.html)",
            1,
            id="versioned_rule_link",
        ),
        pytest.param("`module-variable-prefix`", 0, id="bare_rule_name"),
        pytest.param(
            "[configuration](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html)",
            0,
            id="configuration_link",
        ),
        pytest.param("[documentation](https://github.com/gaborbernat/simplint)", 0, id="external_link"),
    ],
)
def test_changelog_rule_links(tmp_path: Path, capsys: pytest.CaptureFixture[str], line: str, expected: int) -> None:
    path = tmp_path / "CHANGELOG.md"
    path.write_text(f"# Changelog\n{line}\n", encoding="utf-8")

    assert main([str(path)]) == expected
    assert ("use a bare backticked long rule name" in capsys.readouterr().err) == bool(expected)
