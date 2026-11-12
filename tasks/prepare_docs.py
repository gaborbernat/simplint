from __future__ import annotations

import ast
import os
import shlex
import subprocess
import sys
from pathlib import Path


def main(conf: Path, tox: list[str]) -> None:
    # Set up the docs environment and, beside it, mermaid-cli with the browser it draws in, so the build waits on
    # neither. The diagrams need only Chrome's headless shell, not the full Chrome puppeteer would download beside it.
    #
    # :param conf: the Sphinx configuration that pins the mermaid-cli command
    # :param tox: the command that runs tox
    # :raises SystemExit: with the exit code of the setup that failed
    browser = {**os.environ, "PUPPETEER_CHROME_SKIP_DOWNLOAD": "true"}
    command = [*shlex.split(mermaid_command(conf)), "--version"]
    with subprocess.Popen(command, env=browser, stdout=subprocess.DEVNULL) as mermaid:
        setup = subprocess.run([*tox, "run", "-vv", "--notest", "-e", "docs"], check=False).returncode
    if code := setup or mermaid.returncode:
        raise SystemExit(code)


def mermaid_command(conf: Path) -> str:
    # Read the mermaid-cli command the Sphinx configuration pins.
    #
    # :param conf: the Sphinx configuration
    # :returns: the command, as one shell string
    # :raises ValueError: if the configuration sets no ``mermaid_cmd`` string
    for node in ast.parse(conf.read_text(encoding="utf-8")).body:
        if (
            isinstance(node, ast.AnnAssign)
            and isinstance(node.target, ast.Name)
            and node.target.id == "mermaid_cmd"
            and isinstance(node.value, ast.Constant)
            and isinstance(node.value.value, str)
        ):
            return node.value.value
    msg = f"{conf} sets no mermaid_cmd string"
    raise ValueError(msg)


if __name__ == "__main__":
    main(Path(sys.argv[1]), sys.argv[2:])


__all__ = [
    "main",
    "mermaid_command",
]
