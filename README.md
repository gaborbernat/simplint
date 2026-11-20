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
It checks how Python modules organize their definitions, exports and tests. For example, simplint can flag
[helpers defined above their callers](https://simplint.readthedocs.io/en/latest/rules/SMP201.html),
[public definitions missing from `__all__`](https://simplint.readthedocs.io/en/latest/rules/SMP403.html),
[constants missing `Final`](https://simplint.readthedocs.io/en/latest/rules/SMP601.html) and
[tests grouped in classes](https://simplint.readthedocs.io/en/latest/rules/SMP801.html).

Most rules offer a fix. `simplint check --fix` applies safe fixes. Add `--unsafe-fixes` to apply changes that may affect
behavior. Run it over the whole project, including tests, so rules that rename or move definitions can check which
modules use them. Choose the rules that fit your project in `[tool.simplint]` in `pyproject.toml`.

## Install

Install with [uv](https://docs.astral.sh/uv/) or [pipx](https://pipx.pypa.io/stable/). The wheel requires Python 3.11 or
later.

```bash
uv tool install simplint  # or: pipx install simplint
```

## Quick start

Run it from the project root:

```bash
simplint check .
```

## Links

- [Getting started](https://simplint.readthedocs.io/en/latest/tutorial/getting-started.html)
- [How-to guides](https://simplint.readthedocs.io/en/latest/how-to/index.html)
- [Rule reference](https://simplint.readthedocs.io/en/latest/rules.html)
- [PyPI package](https://pypi.org/project/simplint)
- [Release history](https://simplint.readthedocs.io/en/latest/changelog.html)
- [CONTRIBUTING.md](https://github.com/gaborbernat/simplint/blob/main/CONTRIBUTING.md): repository layout and
  development commands
