from __future__ import annotations

import runpy
from pathlib import Path

import pytest

from tasks.assemble_release import assemble, main


@pytest.fixture
def release_tree(tmp_path: Path) -> Path:
    (tmp_path / "CHANGELOG.md").write_text("# Changelog\n\n## 1.0.0 (2026-01-01)\n")
    (tmp_path / "changelog.d").mkdir()
    return tmp_path


def test_assemble_orders_categories_above_the_previous_release(release_tree: Path) -> None:
    fragments = release_tree / "changelog.d"
    (fragments / "+fix.bugfix.md").write_text("Fix an error.\n")
    (fragments / "+feature.feature.md").write_text("Add a\nsetting.\n")

    assemble(release_tree, "1.1.0", "2026-10-21")

    assert (release_tree / "CHANGELOG.md").read_text() == (
        "# Changelog\n\n"
        '## 1.1.0 (2026-10-21)\n\n<a name="release-1-1-0"></a>\n\n'
        '<a name="release-1-1-0-added"></a>\n\n### Added\n\n- Add a setting.\n\n'
        '<a name="release-1-1-0-fixed"></a>\n\n### Fixed\n\n- Fix an error.\n\n'
        "## 1.0.0 (2026-01-01)\n"
    )


def test_assemble_consumes_the_fragments(release_tree: Path) -> None:
    fragment = release_tree / "changelog.d" / "123.feature.md"
    fragment.write_text("Add a setting.\n")

    assemble(release_tree, "1.1.0", "2026-10-21")

    assert not fragment.exists()


@pytest.mark.parametrize(
    ("filename", "content", "message"),
    [
        pytest.param("oops.md", "Add a setting.", "invalid changelog fragment", id="no-category"),
        pytest.param("+name.other.md", "Add a setting.", "invalid changelog fragment", id="unknown-category"),
        pytest.param("name.feature.md", "Add a setting.", "invalid changelog fragment", id="bad-stem"),
        pytest.param("+name.feature.md", "", "expected one nonempty paragraph", id="empty"),
        pytest.param("+name.feature.md", "One.\n\nTwo.", "expected one nonempty paragraph", id="two-paragraphs"),
        pytest.param("+name.feature.md", "- Existing bullet.", "expected one nonempty paragraph", id="bullet"),
    ],
)
def test_assemble_rejects_invalid_fragments(release_tree: Path, filename: str, content: str, message: str) -> None:
    (release_tree / "changelog.d" / filename).write_text(content)

    with pytest.raises(ValueError, match=message):
        assemble(release_tree, "1.1.0", "2026-10-21")


def test_assemble_requires_the_changelog_heading(release_tree: Path) -> None:
    (release_tree / "CHANGELOG.md").write_text("## 1.0.0\n")

    with pytest.raises(ValueError, match="must start with"):
        assemble(release_tree, "1.1.0", "2026-10-21")


def test_main_dates_the_release(release_tree: Path) -> None:
    (release_tree / "changelog.d" / "+feature.feature.md").write_text("Add a setting.\n")

    main(["1.1.0", "--root", str(release_tree)])

    assert "## 1.1.0 (" in (release_tree / "CHANGELOG.md").read_text()


def test_command_assembles_release(release_tree: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    (release_tree / "changelog.d" / "+feature.feature.md").write_text("Add a setting.\n")
    monkeypatch.setattr("sys.argv", ["tasks/assemble_release.py", "1.1.0", "--root", str(release_tree)])

    runpy.run_path(str(Path(__file__).parents[2] / "tasks" / "assemble_release.py"), run_name="__main__")

    assert "## 1.1.0 (" in (release_tree / "CHANGELOG.md").read_text()
