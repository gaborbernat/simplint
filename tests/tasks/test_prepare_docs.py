from __future__ import annotations

import shlex
import sys
from pathlib import Path

import pytest

from tasks.prepare_docs import main, mermaid_command


def test_main_sets_up_docs_and_mermaid_with_the_headless_shell_only(tmp_path: Path) -> None:
    conf = _conf(
        tmp_path, "import pathlib; pathlib.Path('browser').write_text(os.environ['PUPPETEER_CHROME_SKIP_DOWNLOAD'])"
    )
    tox = f"import pathlib, sys; pathlib.Path({str(tmp_path / 'tox')!r}).write_text(' '.join(sys.argv[1:]))"

    main(conf, [sys.executable, "-c", tox])

    assert ((tmp_path / "browser").read_text(), (tmp_path / "tox").read_text()) == ("true", "run -vv --notest -e docs")


def _conf(folder: Path, script: str) -> Path:
    # The test processes write into the folder, so it is their working directory.
    conf = folder / "conf.py"
    command = shlex.join([sys.executable, "-c", f"import os; os.chdir({str(folder)!r}); {script}"])
    conf.write_text(f"mermaid_cmd: str = {command!r}\n", encoding="utf-8")
    return conf


@pytest.mark.parametrize(
    ("mermaid", "setup", "code"), [pytest.param(0, 3, 3, id="tox"), pytest.param(4, 0, 4, id="mermaid")]
)
def test_main_fails_when_either_setup_fails(tmp_path: Path, mermaid: int, setup: int, code: int) -> None:
    conf = _conf(tmp_path, f"raise SystemExit({mermaid})")

    with pytest.raises(SystemExit) as raised:
        main(conf, [sys.executable, "-c", f"raise SystemExit({setup})"])

    assert raised.value.code == code


def test_mermaid_command_reads_the_pinned_command() -> None:
    conf = Path(__file__).parents[2] / "docs" / "conf.py"

    assert mermaid_command(conf).startswith("npx --yes --package=@mermaid-js/mermaid-cli@")


def test_mermaid_command_rejects_a_configuration_without_it(tmp_path: Path) -> None:
    conf = tmp_path / "conf.py"
    conf.write_text('mermaid_cmd = "npx"\nother: str = "npx"\n', encoding="utf-8")

    with pytest.raises(ValueError, match=r"sets no mermaid_cmd string$"):
        mermaid_command(conf)
