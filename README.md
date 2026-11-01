<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/_static/wordmark-dark.png">
    <img alt="simplint" src="docs/_static/wordmark-light.png" width="420">
  </picture>
</p>

# simplint

[![PyPI](https://img.shields.io/pypi/v/simplint?style=flat-square)](https://pypi.org/project/simplint)
[![PyPI - Python Version](https://img.shields.io/pypi/pyversions/simplint?style=flat-square)](https://pypi.org/project/simplint)
[![Downloads](https://static.pepy.tech/badge/simplint/month)](https://pepy.tech/project/simplint)
[![PyPI - License](https://img.shields.io/pypi/l/simplint?style=flat-square)](https://opensource.org/licenses/MIT)
[![Build](https://github.com/gaborbernat/simplint/actions/workflows/build.yaml/badge.svg)](https://github.com/gaborbernat/simplint/actions/workflows/build.yaml)
[![Test](https://github.com/gaborbernat/simplint/actions/workflows/test.yaml/badge.svg)](https://github.com/gaborbernat/simplint/actions/workflows/test.yaml)
[![Documentation status](https://readthedocs.org/projects/simplint/badge/?version=latest)](https://simplint.readthedocs.io/en/latest/)

simplint is a companion linter for [Ruff](https://docs.astral.sh/ruff/), written in [Rust](https://www.rust-lang.org/).
It adds checks that make Python code easier to read and review, and fixes what it reports when the fix is mechanical.
The checks cover
[definitions ordered like a book](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP020.html),
[explicit exports](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP103.html),
[flat test modules](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP200.html) and more. The rules are
opinionated. They reflect the maintainer's preferences while aiming to be as objective as possible.

Install it with `uv tool install simplint` ([uv](https://docs.astral.sh/uv/)) or `pipx install simplint`
([pipx](https://pipx.pypa.io/stable/)), then run `simplint check .` from the project root.

- [Documentation](https://simplint.readthedocs.io/en/latest/): installation, quick start and
  [pre-commit](https://pre-commit.com/) hook
- [Configuration](https://simplint.readthedocs.io/en/latest/configuration.html)
- [Rules](https://simplint.readthedocs.io/en/latest/rules.html)
- [PyPI package](https://pypi.org/project/simplint)
- [Release history](CHANGELOG.md)
- [CONTRIBUTING.md](CONTRIBUTING.md): repository layout and development commands
