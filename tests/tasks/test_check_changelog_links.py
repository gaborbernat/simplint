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
            "[`module-variable-prefix`](https://simplint.readthedocs.io/en/latest/rules/SMP404.html)",
            1,
            id="latest_rule_link",
        ),
        pytest.param(
            "[rule](http://simplint.readthedocs.io/en/0.10.0/rules/SMP101.html)",
            1,
            id="versioned_rule_link",
        ),
        pytest.param("`module-variable-prefix`", 0, id="bare_rule_name"),
        pytest.param("[`all-format`](src/rules/api/all_format.md)", 0, id="relative_source_link"),
        pytest.param(
            "[`all-format`](https://github.com/gaborbernat/simplint/blob/1.0.0/src/rules/api/all_format.md)",
            0,
            id="release_source_link",
        ),
        pytest.param("[all-format](src/rules/api/all_format.md)", 1, id="unquoted_source_link"),
        pytest.param("[`all-format`](src/rules/api/missing_from_all.md)", 1, id="mismatched_source_link"),
        pytest.param("[`all-format`](src/rules/other/all_format.md)", 1, id="missing_source_link"),
        pytest.param("[`missing-from-all`](src/rules/api/missing_from_all.md)", 1, id="unknown_rule"),
        pytest.param(
            "[configuration](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html)",
            0,
            id="configuration_link",
        ),
        pytest.param("[documentation](https://github.com/gaborbernat/simplint)", 0, id="external_link"),
    ],
)
@pytest.mark.usefixtures("registry")
def test_changelog_rule_links(tmp_path: Path, capsys: pytest.CaptureFixture[str], line: str, expected: int) -> None:
    (tmp_path / "src" / "rules" / "api").mkdir(parents=True)
    (tmp_path / "src" / "rules" / "api" / "all_format.md").touch()
    (tmp_path / "src" / "rules" / "api" / "missing_from_all.md").touch()
    path = tmp_path / "CHANGELOG.md"
    path.write_text(f"# Changelog\n{line}\n", encoding="utf-8")

    assert main([str(path)]) == expected
    assert bool(capsys.readouterr().err) == bool(expected)


@pytest.mark.parametrize(
    ("line", "expected"),
    [
        pytest.param("[`all-format`](../src/rules/api/all_format.md)", 0, id="quoted"),
        pytest.param("[all-format](../src/rules/api/all_format.md)", 1, id="unquoted"),
    ],
)
@pytest.mark.usefixtures("registry")
def test_fragment_rule_link(tmp_path: Path, line: str, expected: int) -> None:
    (tmp_path / "changelog.d").mkdir()
    (tmp_path / "src" / "rules" / "api").mkdir(parents=True)
    (tmp_path / "src" / "rules" / "api" / "all_format.md").touch()
    fragment = tmp_path / "changelog.d" / "+rule.bugfix.md"
    fragment.write_text(f"{line}\n", encoding="utf-8")

    assert main([str(fragment)]) == expected


@pytest.mark.usefixtures("registry")
def test_multiple_rule_links_read_the_registry_once(tmp_path: Path) -> None:
    (tmp_path / "src" / "rules" / "api").mkdir(parents=True)
    (tmp_path / "src" / "rules" / "api" / "all_format.md").touch()
    path = tmp_path / "CHANGELOG.md"
    path.write_text("[`all-format`](src/rules/api/all_format.md) twice: [`all-format`](src/rules/api/all_format.md)\n")

    assert main([str(path)]) == 0
