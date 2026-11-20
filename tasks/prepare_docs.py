from __future__ import annotations

import ast
import os
import shlex
import subprocess
import sys
from pathlib import Path


def main(conf: Path, tox: list[str]) -> None:
    # Set up tox and mermaid-cli in parallel. The renderer uses Chrome's headless shell, so skip the full browser.
    browser = {**os.environ, "PUPPETEER_CHROME_SKIP_DOWNLOAD": "true"}
    command = [*shlex.split(mermaid_command(conf)), "--version"]
    with subprocess.Popen(command, env=browser, stdout=subprocess.DEVNULL) as mermaid:
        setup = subprocess.run([*tox, "run", "-vv", "--notest", "-e", "docs"], check=False).returncode
    if code := setup or mermaid.returncode:
        raise SystemExit(code)


def mermaid_command(conf: Path) -> str:
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
