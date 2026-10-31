from __future__ import annotations

import subprocess
from typing import TYPE_CHECKING

import pytest

from tasks.generate_changelog import link_rules, main, rule_names

if TYPE_CHECKING:
    from pathlib import Path

    from pytest_mock import MockerFixture


def test_main_attaches_anchors_and_shows_pending_news(tmp_path: Path, mocker: MockerFixture) -> None:
    (tmp_path / "CHANGELOG.md").write_text(
        '# Changelog\n\n## 1.0.0 (2026-01-01)\n\n<a name="release-1-0-0"></a>\n\n'
        '<a name="release-1-0-0-added"></a>\n\n### Added\n\n- First.\n'
    )
    (tmp_path / "changelog.d").mkdir()
    (tmp_path / "changelog.d" / "+next.bugfix.md").write_text("Fix it.\n")
    (tmp_path / "docs").mkdir()
    convert = mocker.patch("pypandoc.convert_text", autospec=True, return_value="rendered")

    main(tmp_path, {})

    assert convert.call_args.args[0] == (
        "# Changelog\n\n## Draft {#release-draft}\n\n\n\n### Fixed {#release-draft-fixed}\n\n- Fix it.\n\n\n"
        "## 1.0.0 (2026-01-01) {#release-1-0-0}\n\n\n\n### Added {#release-1-0-0-added}\n\n- First."
    )


def test_main_rejects_an_anchor_without_a_heading(tmp_path: Path) -> None:
    (tmp_path / "CHANGELOG.md").write_text(
        '# Changelog\n\n<a name="release-1-0-0"></a>\n\n<a name="release-1-0-0-added"></a>\n'
    )
    (tmp_path / "changelog.d").mkdir()

    with pytest.raises(ValueError, match="cannot attach changelog anchor"):
        main(tmp_path, {})


def test_link_rules_names_each_rule_by_its_long_name() -> None:
    rules = {"SMP115": "all-format", "SMP113": "missing-from-all"}

    assert link_rules("`all-format` fixes `__all__`, like SMP113.", rules) == (
        "`` :doc:`all-format <_generated/rules/SMP115>` ``{=rst} fixes `__all__`, like "
        "`` :doc:`missing-from-all <_generated/rules/SMP113>` ``{=rst}."
    )


def test_rule_names_reads_the_installed_binary(mocker: MockerFixture) -> None:
    mocker.patch("shutil.which", autospec=True, return_value="/bin/simplint")
    mocker.patch(
        "subprocess.run",
        autospec=True,
        return_value=subprocess.CompletedProcess([], 0, stdout='[{"code": "SMP115", "name": "all-format"}]'),
    )

    assert rule_names() == {"SMP115": "all-format"}


def test_rule_names_without_the_binary_links_nothing(mocker: MockerFixture) -> None:
    mocker.patch("shutil.which", autospec=True, return_value=None)

    assert rule_names() == {}
