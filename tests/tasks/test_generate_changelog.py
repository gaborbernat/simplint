from __future__ import annotations

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


def test_main_links_codes_only_in_pending_news(tmp_path: Path, mocker: MockerFixture) -> None:
    (tmp_path / "CHANGELOG.md").write_text("# Changelog\n\n## 1.0.0 (2026-01-01)\n\n- SMP101 meant `missing-final`.\n")
    (tmp_path / "changelog.d").mkdir()
    (tmp_path / "changelog.d" / "+next.bugfix.md").write_text("SMP601 and `missing-final` now agree.\n")
    (tmp_path / "docs").mkdir()
    convert = mocker.patch("pypandoc.convert_text", autospec=True, return_value="rendered")

    main(tmp_path, {"SMP101": "single-letter-name", "SMP601": "missing-final"})

    assert convert.call_args.args[0] == (
        "# Changelog\n\n## Draft {#release-draft}\n\n\n\n### Fixed {#release-draft-fixed}\n\n"
        "- `` :doc:`missing-final <rules/SMP601>` ``{=rst} and "
        "`` :doc:`missing-final <rules/SMP601>` ``{=rst} now agree.\n\n\n"
        "## 1.0.0 (2026-01-01)\n\n- SMP101 meant `` :doc:`missing-final <rules/SMP601>` ``{=rst}."
    )


def test_main_keeps_the_anchor_of_a_release_without_entries(tmp_path: Path, mocker: MockerFixture) -> None:
    (tmp_path / "CHANGELOG.md").write_text(
        '# Changelog\n\n## 1.0.1 (2026-01-02)\n\n<a name="release-1-0-1"></a>\n\n'
        '## 1.0.0 (2026-01-01)\n\n<a name="release-1-0-0"></a>\n\n'
        '<a name="release-1-0-0-added"></a>\n\n### Added\n\n- First.\n'
    )
    (tmp_path / "changelog.d").mkdir()
    (tmp_path / "docs").mkdir()
    convert = mocker.patch("pypandoc.convert_text", autospec=True, return_value="rendered")

    main(tmp_path, {})

    assert convert.call_args.args[0] == (
        "# Changelog\n\n## 1.0.1 (2026-01-02) {#release-1-0-1}\n\n\n"
        "## 1.0.0 (2026-01-01) {#release-1-0-0}\n\n\n\n### Added {#release-1-0-0-added}\n\n- First."
    )


def test_main_rejects_an_anchor_without_a_heading(tmp_path: Path) -> None:
    (tmp_path / "CHANGELOG.md").write_text(
        '# Changelog\n\n<a name="release-1-0-0"></a>\n\n<a name="release-1-0-0-added"></a>\n'
    )
    (tmp_path / "changelog.d").mkdir()

    with pytest.raises(ValueError, match="cannot attach changelog anchor"):
        main(tmp_path, {})


def test_link_rules_links_codes_and_names() -> None:
    rules = {"SMP405": "all-format", "SMP401": "missing-from-all"}

    assert link_rules("`all-format` fixes `__all__`, like SMP401.", rules, codes=True) == (
        "`` :doc:`all-format <rules/SMP405>` ``{=rst} fixes `__all__`, like "
        "`` :doc:`missing-from-all <rules/SMP401>` ``{=rst}."
    )


def test_link_rules_drops_code_after_linked_name() -> None:
    rules = {"SMP405": "all-format", "SMP401": "missing-from-all"}

    assert link_rules("`all-format` (SMP405) fixes `__all__`, like SMP401.", rules, codes=True) == (
        "`` :doc:`all-format <rules/SMP405>` ``{=rst} fixes `__all__`, like "
        "`` :doc:`missing-from-all <rules/SMP401>` ``{=rst}."
    )


def test_link_rules_leaves_codes_plain_without_codes() -> None:
    rules = {"SMP405": "all-format", "SMP401": "missing-from-all"}

    assert link_rules("`all-format` fixes `__all__`, like SMP401.", rules, codes=False) == (
        "`` :doc:`all-format <rules/SMP405>` ``{=rst} fixes `__all__`, like SMP401."
    )


@pytest.mark.parametrize("prefix", ["", "../", "https://github.com/gaborbernat/simplint/blob/1.0.0/"])
def test_link_rules_resolves_relative_source_links(prefix: str) -> None:
    markdown = f"[`all-format`]({prefix}src/rules/api/all_format.md) (SMP405)"

    assert link_rules(markdown, {"SMP405": "all-format"}, codes=True) == (
        "`` :doc:`all-format <rules/SMP405>` ``{=rst}"
    )


@pytest.mark.parametrize("prefix", ["", "../", "https://github.com/gaborbernat/simplint/blob/1.0.0/"])
def test_link_rules_links_documentation_pages_to_the_docs(prefix: str) -> None:
    markdown = f"A [design principles]({prefix}docs/explanation/design-principles.rst) page"

    assert link_rules(markdown, {}, codes=False) == (
        "A `` :doc:`design principles </explanation/design-principles>` ``{=rst} page"
    )


def test_link_rules_rejects_mismatched_source_link() -> None:
    with pytest.raises(ValueError, match="rule source link for all-format points to"):
        link_rules("[`all-format`](src/rules/api/missing_from_all.md)", {"SMP405": "all-format"}, codes=True)


def test_link_rules_rejects_unknown_source_link() -> None:
    with pytest.raises(ValueError, match="unknown rule source link: missing-from-all"):
        link_rules("[`missing-from-all`](src/rules/api/missing_from_all.md)", {"SMP405": "all-format"}, codes=True)


@pytest.mark.usefixtures("registry")
def test_rule_names_reads_the_rule_sources(tmp_path: Path) -> None:
    (tmp_path / "src" / "rules" / "api").mkdir(parents=True)
    (tmp_path / "src" / "rules" / "api" / "all_format.md").touch()

    assert rule_names(tmp_path) == {"SMP405": "all-format"}
