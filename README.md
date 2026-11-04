<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/gaborbernat/simplint/main/docs/_static/wordmark-dark.png">
    <img alt="simplint" src="https://raw.githubusercontent.com/gaborbernat/simplint/main/docs/_static/wordmark-light.png" width="420">
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
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/gaborbernat/simplint/badge)](https://scorecard.dev/viewer/?uri=github.com/gaborbernat/simplint)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/16281/badge)](https://www.bestpractices.dev/projects/16281)

simplint is a companion linter for [Ruff](https://docs.astral.sh/ruff/), written in [Rust](https://www.rust-lang.org/).
It adds checks that make Python code easier to read and review:

- [definitions ordered like a book](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP201.html)
- [explicit exports](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP403.html)
- [Final constants](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP601.html)
- [flat test modules](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP801.html)
- and more

Most rules rewrite the code for you; fixes that could change behavior run only with `--unsafe-fixes`. An import index
across every file keeps fixes from renaming or moving a name another module uses. A whole project lints in milliseconds,
few rules overlap with Ruff's, and both tools read the same `noqa` comments and `pyproject.toml`.

The rules are opinionated. They reflect the maintainer's preferences while aiming to be as objective as possible.

## Install

With [uv](https://docs.astral.sh/uv/) or [pipx](https://pipx.pypa.io/stable/):

```bash
uv tool install simplint  # or: pipx install simplint
```

## Quick start

Run it from the project root:

```bash
simplint check .
```

## Links

- [Documentation](https://simplint.readthedocs.io/en/latest/): start with the
  [getting started](https://simplint.readthedocs.io/en/latest/tutorial/getting-started.html) tutorial, then the
  [how-to guides](https://simplint.readthedocs.io/en/latest/how-to/index.html) for single tasks, and the
  [reference](https://simplint.readthedocs.io/en/latest/rules.html) for rules, configuration and the command line
- [PyPI package](https://pypi.org/project/simplint)
- [Release history](https://simplint.readthedocs.io/en/latest/changelog.html)
- [CONTRIBUTING.md](https://github.com/gaborbernat/simplint/blob/main/CONTRIBUTING.md): repository layout and
  development commands
