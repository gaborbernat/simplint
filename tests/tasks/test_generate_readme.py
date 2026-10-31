from __future__ import annotations

from typing import TYPE_CHECKING

from tasks.generate_readme import main, process_rst_for_pypi, strip_main_title, unwrap_dropdowns

if TYPE_CHECKING:
    from pathlib import Path


def test_process_rst_for_pypi_rewrites_roles() -> None:
    assert process_rst_for_pypi(
        ":pypi:`simplint` and :gh:`astral-sh/ruff`\nSee :doc:`configuration` for details.\n"
    ) == ("`simplint <https://pypi.org/project/simplint>`_ and `astral-sh/ruff <https://github.com/astral-sh/ruff>`_")


def test_process_rst_for_pypi_keeps_only_the_uv_tab() -> None:
    source = ".. tab:: uv\n\n    uv tool install x\n\n.. tab:: pip\n\n    pip install x\n\nAfter\n"

    assert process_rst_for_pypi(source) == "\n    uv tool install x\n\nAfter"


def test_process_rst_for_pypi_drops_the_command_line_section() -> None:
    source = "Intro\n\nCommand line interface\n----------------------\n\nhelp\n\nNext\n----\n\nKept\n"

    assert process_rst_for_pypi(source) == "Intro\n\nNext\n----\n\nKept"


def test_unwrap_dropdowns_dedents_the_body() -> None:
    assert unwrap_dropdowns(".. dropdown:: Title\n\n    Body\nAfter") == "\nBody\nAfter"


def test_strip_main_title_keeps_untitled_content() -> None:
    assert strip_main_title("No title\n") == "No title\n"


def test_main_writes_the_readme_from_the_docs(tmp_path: Path) -> None:
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "index.rst").write_text("simplint\n========\n\nIntro\n")
    (tmp_path / "docs" / "configuration.rst").write_text("Configuration\n=============\n\nSettings\n")

    main(tmp_path)

    assert (tmp_path / "README.rst").read_text() == (
        ".. image:: https://raw.githubusercontent.com/gaborbernat/simplint/main/docs/_static/wordmark-light.png\n"
        "   :alt: simplint\n   :width: 420\n\nsimplint\n========\n\nIntro\n\nSettings\n"
    )


def test_process_rst_for_pypi_drops_html_and_grids() -> None:
    source = ".. raw:: html\n\n    <section>hero</section>\n\n.. grid:: 2\n\n    .. grid-item-card:: Fast\n\nKept\n"

    assert process_rst_for_pypi(source) == "Kept"
