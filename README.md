# simplint

simplint is a companion linter for [Ruff](https://github.com/astral-sh/ruff). It enforces the parts of the `simp` house
style that Ruff has no rule for, such as definition order, `Final` constants, explicit exports and test layout. It is
built on Ruff's own parser crates, so the 264 files of [tox](https://github.com/tox-dev/tox) lint in about 10 ms, and it
fixes what it reports wherever the fix is mechanical.

```console
$ simplint check --select SMP050,SMP101 src/tox/util/path.py src/tox/util/spinner.py
src/tox/util/path.py:16:1: SMP050 `_CACHEDIR_TAG` is used by one function only; define it there
src/tox/util/path.py:16:1: SMP101 [*] `_CACHEDIR_TAG` is never reassigned; annotate it `Final`
src/tox/util/spinner.py:41:1: SMP101 [*] `MISS_DURATION` is never reassigned; annotate it `Final`
src/tox/util/spinner.py:181:1: SMP050 `_PERIODS` is used by one function only; define it there
src/tox/util/spinner.py:181:1: SMP101 `_PERIODS` is never reassigned; annotate it `Final`
Found 5 issues.
[*] 2 fixable with `--fix`.
2 hidden fixes can be enabled with `--unsafe-fixes`.
```

## Installation

simplint ships as a single binary inside a Python wheel, like Ruff. Install it as a tool with
`uv tool install simplint`, or run it without installing with `uvx simplint check .`; `pipx` and `pip` work as well. The
[documentation](https://simplint.readthedocs.io) explains every rule.

## Running it

`simplint check [PATHS]` lints the Python files under each path, honoring `.gitignore`. Add `--fix` to apply safe fixes,
`--unsafe-fixes` to also apply fixes that can change behavior, and `--diff` to print the fixes without writing them.
`--select` and `--ignore` take comma-separated rule codes, code prefixes or rule names. `simplint rules` lists every
rule and whether it fixes.

Fixes rewrite code without formatting it, so run the formatter afterwards, the same way you would after
`ruff check --fix`:

```console
simplint check --fix . && ruff check --fix . && ruff format .
```

## Configuration

simplint reads the nearest `pyproject.toml`. From `[tool.ruff]` it takes `target-version` (falling back to
`project.requires-python`), `exclude` and `extend-exclude`. Its own settings live in `[tool.simplint]`:

```toml
[tool.simplint]
select = ["ALL"]                              # the default
extend-select = []
ignore = ["SMP002"]                           # codes, prefixes or names
per-file-ignores = { "tests/*" = ["SMP101"] }
```

Suppress a single line with `# noqa: SMP010`. Tell Ruff those codes belong to another tool so it does not report them as
unused:

```toml
[tool.ruff]
lint.external = ["SMP"]
```

## Pairing with Ruff

simplint does not repeat what Ruff already checks. Run Ruff with `select = ["ALL"]` and add this configuration to cover
the rest of the style:

```toml
[tool.ruff.lint.flake8-tidy-imports]
ban-relative-imports = "parents"

[tool.ruff.lint.flake8-tidy-imports.banned-api]
"typing.Any".msg = "use the precise type"
"typing.cast".msg = "narrow with isinstance or TypeIs instead"
```

Ruff reads `banned-api` per configuration file, so the test-only bans go in a `ruff.toml` inside the tests directory. It
extends the root configuration and repeats the bans above, since a nested table replaces the parent's:

```toml
extend = "../pyproject.toml"

[lint.flake8-tidy-imports.banned-api]
"typing.Any".msg = "use the precise type"
"typing.cast".msg = "narrow with isinstance or TypeIs instead"
"unittest.mock".msg = "mock through pytest-mock's mocker"
"time.sleep".msg = "wait on an event the mock sets"
```

## How it decides

A few rules need to know more than one file. simplint first indexes the whole run: which names each module imports from
the others, including `module.attr` access, dotted strings such as `mocker.patch("pkg.mod.NAME")`, and the entry points
in `pyproject.toml`. A name another module imports is never renamed or moved, and must appear in `__all__` (SMP113). The
index covers only the files in the run, so check the whole project, tests included, before trusting these rules.

Fixes come in two strengths. A **safe** fix keeps behavior. An **unsafe** fix, applied only with `--unsafe-fixes`, can
change it. The ones to watch:

- SMP114 adds `kw_only=True`, which breaks callers that build the dataclass positionally.
- SMP111 adds `frozen=True, slots=True`. The fix is safe only when no file in the run assigns or deletes an attribute
  named like one of the fields, the class neither inherits nor is inherited from, and its body neither sets attributes
  nor uses `super`, `cached_property`, `__dict__`, `__weakref__`, `__slots__`, `__setattr__` or `__delattr__`. A class
  that already sets one of the options to `False` gets an unsafe fix. The check cannot see code that adds a new
  attribute to an instance from outside the class.
- SMP102 and SMP103 rename module-level names. They skip any name another file in the run imports, but cannot see
  importers outside it.
- SMP050 moves a constant into its only function, which then rebuilds it on every call.
- SMP020 moves decorated functions and classes, which changes decorator registration order.

A rule reports without a fix when the fix cannot be done safely in text: the statement shares its line with another, a
string spans several lines, or the new name would collide with an existing one.

## Rules

`simplint rules` lists every rule, `simplint rule SMP111` explains one, and the
[rules reference](https://simplint.readthedocs.io/en/latest/rules.html) documents all of them with examples and fix
safety.
