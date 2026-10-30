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

## Rules

| Code   | Name                       | Checks                                                      | Fix    |
| ------ | -------------------------- | ----------------------------------------------------------- | ------ |
| SMP001 | section-comment            | comments used as dividers or section headers                | yes    |
| SMP002 | function-docstring         | docstrings on functions and classes                         | unsafe |
| SMP003 | multiline-module-docstring | module docstrings longer than one line                      |        |
| SMP004 | test-module-docstring      | docstrings on test modules                                  | yes    |
| SMP005 | long-paragraph             | comment or docstring paragraphs over 100 words              |        |
| SMP010 | single-letter-name         | single-letter names outside loop and comprehension targets  |        |
| SMP020 | definition-before-use      | helpers defined above their first caller, or away from it   | yes    |
| SMP030 | forwarding-function        | private functions that only forward to another call         |        |
| SMP040 | dependency-import-guard    | `try`/`except ImportError` around a required dependency     | unsafe |
| SMP050 | single-user-constant       | module constants only one function uses                     | unsafe |
| SMP060 | suppression-without-reason | `noqa`, `type: ignore` and `pragma` comments with no reason |        |
| SMP100 | walrus-candidate           | an assignment the next `if` test could absorb with `:=`     | yes    |
| SMP101 | missing-final              | module constants not annotated `Final`                      | yes    |
| SMP102 | module-variable-case       | module variables not `UPPER_CASE`, or wrongly `_`-prefixed  | unsafe |
| SMP103 | unexported-public-name     | public functions and classes missing from `__all__`         | unsafe |
| SMP104 | all-not-last               | `__all__` not at the end of the module                      | yes    |
| SMP105 | callable-ellipsis          | `Callable[..., T]`                                          |        |
| SMP106 | object-annotation          | `object` annotations outside `TypeIs` guards and dunders    |        |
| SMP107 | vars-call                  | `vars()` calls                                              |        |
| SMP108 | future-annotations         | `from __future__ import annotations` on Python 3.14+        | yes    |
| SMP109 | init-module-content        | `__init__.py` holding more than a docstring                 |        |
| SMP110 | absolute-sibling-import    | absolute imports from the same package                      | yes    |
| SMP111 | dataclass-options          | dataclasses without `frozen` and `slots`                    | yes    |
| SMP112 | string-concatenation       | `+` between strings                                         | unsafe |
| SMP113 | missing-from-all           | names other modules import that `__all__` omits             | yes    |
| SMP114 | dataclass-kw-only          | dataclasses without `kw_only`                               | unsafe |
| SMP200 | test-class                 | test classes                                                | unsafe |
| SMP201 | parametrize-without-id     | `parametrize` rows without `pytest.param(..., id=...)`      |        |
| SMP202 | conditional-in-test        | `if`, `match` and conditional expressions in tests          |        |
| SMP203 | mock-attribute-assignment  | mock attributes assigned after construction                 | yes    |
| SMP204 | mock-without-spec          | mocks and patches without a spec                            |        |
| SMP205 | constant-assert            | assertions on a truthy constant                             | yes    |

A rule reports without a fix when the fix cannot be done safely in text: the statement shares its line with another, a
string spans several lines, or the new name would collide with an existing one.
