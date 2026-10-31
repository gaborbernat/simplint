from __future__ import annotations

import re
from pathlib import Path
from typing import Final

_ROOT: Final[Path] = Path(__file__).resolve().parents[1]


def main(root: Path) -> None:
    """
    Write ``README.rst`` for PyPI from the index and configuration docs pages.

    :param root: the project root
    """
    docs_dir = root / "docs"
    # PyPI renders the README on a light page and cannot resolve repository-relative image paths
    logo = (
        ".. image:: https://raw.githubusercontent.com/gaborbernat/simplint/main/docs/_static/wordmark-light.png\n"
        "   :alt: simplint\n   :width: 420\n\n"
    )
    processed = logo + process_rst_for_pypi((docs_dir / "index.rst").read_text(encoding="utf-8"))
    configuration = (docs_dir / "configuration.rst").read_text(encoding="utf-8")
    processed += f"\n\n{process_rst_for_pypi(strip_main_title(configuration))}"
    (root / "README.rst").write_text(f"{processed.rstrip()}\n", encoding="utf-8")


def process_rst_for_pypi(content: str) -> str:
    """
    Rewrite Sphinx-only reStructuredText into markup PyPI can render.

    :param content: the Sphinx source
    :returns: the content with roles turned into links and unsupported directives and the CLI section dropped
    """
    content = unwrap_dropdowns(content)
    pypi_role = re.compile(
        r""":pypi:`(?P<target>[^`]+)`""",  # Link text is also the PyPI project path.
        re.VERBOSE,
    )
    content = pypi_role.sub(r"`\g<target> <https://pypi.org/project/\g<target>>`_", content)
    gh_role = re.compile(
        r""":gh:`(?P<target>[^`]+)`""",  # Link text is also the GitHub path.
        re.VERBOSE,
    )
    content = gh_role.sub(r"`\g<target> <https://github.com/\g<target>>`_", content)
    doc_reference = re.compile(
        r"""^See\ :doc:`[^`]+`.*$\n?""",  # The standalone docs link has no PyPI target.
        re.MULTILINE | re.VERBOSE,
    )
    content = doc_reference.sub("", content)
    result: list[str] = []
    skip_section = False
    skip_tab = False
    lines = content.splitlines()
    for i, line in enumerate(lines):
        if (
            "Command line interface" in line
            and i + 1 < len(lines)
            and (next_line := lines[i + 1])
            and all(c in "-~=" for c in next_line)
        ):
            skip_section = True
            continue
        if skip_section:
            # the section runs until the next heading, a text line over an underline
            if not (
                (line and not all(c in "-~=" for c in line) and i + 1 < len(lines) and lines[i + 1])
                and all(c in "-~=" for c in lines[i + 1])
            ):
                continue
            skip_section = False
        if line.startswith(".. tab:: uv"):
            continue
        if line.startswith((".. tab::", ".. toctree::", ".. sphinx_argparse_cli::", ".. raw::", ".. grid::")):
            skip_tab = True
            continue
        if skip_tab:
            if line and not line.startswith(" ") and not line.startswith("\t"):
                skip_tab = False
            else:
                continue
        result.append(line)
    return "\n".join(result).rstrip()


def unwrap_dropdowns(content: str) -> str:
    """
    Replace each dropdown directive with its dedented body.

    :param content: the reStructuredText source
    :returns: the content without dropdown wrappers
    """
    result: list[str] = []
    in_dropdown = False
    for line in content.splitlines():
        if not in_dropdown:
            if line.startswith(".. dropdown::"):
                in_dropdown = True
            else:
                result.append(line)
        elif not line:
            result.append("")
        elif line.startswith("    "):
            result.append(line[4:])
        else:
            in_dropdown = False
            result.append(line)
    return "\n".join(result)


def strip_main_title(content: str) -> str:
    """
    Drop the top-level title of a page.

    :param content: the reStructuredText source
    :returns: the content after its ``=`` underlined title, or the content unchanged when it has none
    """
    lines = content.splitlines()
    match lines:
        case [_, underline, *body] if underline and set(underline) == {"="}:
            return "\n".join(body).lstrip()
        case _:
            return content


if __name__ == "__main__":
    main(_ROOT)


__all__ = [
    "main",
    "process_rst_for_pypi",
    "strip_main_title",
    "unwrap_dropdowns",
]
