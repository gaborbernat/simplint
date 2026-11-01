## What it does

Checks that module-scope variable names match their visibility: `UPPER_CASE` for names listed in
[`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package), `_UPPER_CASE` for the rest. In a
module without `__all__`, every variable counts as private and should be `_UPPER_CASE`. A name another module in the run
imports keeps its current leading underscore, and only its case changes.

simplint reads `__all__` from assignments, `+=`, `.extend([...])` and `.append("name")` with string literals. When any
other statement builds it, such as `list(...)` or `.remove(...)`, or when it is bound inside a nested block such as an
`if`, `try` or function, the rule is skipped.

Names bound with `:=` are checked. The rule skips:

- dunder names
- class-like names, such as the type alias `Alias = int`
- names also bound by an import, `def` or `class`
- names bound only as `for`, `with`, `except` or `match` targets
- pytest's [`pytestmark`](https://docs.pytest.org/en/stable/example/markers.html#marking-whole-classes-or-modules),
  [`pytest_plugins`](https://docs.pytest.org/en/stable/how-to/plugins.html#requiring-loading-plugins-in-a-test-module-or-conftest-file),
  [`collect_ignore` and `collect_ignore_glob`](https://docs.pytest.org/en/stable/example/pythoncollection.html#customizing-test-collection)
- `TYPE_CHECKING`, since type checkers recognize the guard by that name whatever its value
- every name in a `conf.py`, since [Sphinx](https://www.sphinx-doc.org/en/master/usage/configuration.html) reads its
  settings by their lowercase names

Ruff's [mixed-case-variable-in-global-scope](https://docs.astral.sh/ruff/rules/mixed-case-variable-in-global-scope/)
(N816) flags only `mixedCase` names. This rule also flags `lower_case` names and a wrong leading underscore.
[SMP103](SMP103.html) applies the same underscore convention to functions and classes.

## Why is this bad?

Upper case tells a reader the name is a module-level value rather than a local, as
[PEP 8](https://peps.python.org/pep-0008/#constants) writes constants. The leading underscore tells them whether other
modules may use it ([PEP 8](https://peps.python.org/pep-0008/#public-and-internal-interfaces)). Deriving both from
`__all__` keeps the names in line with what the module exports.

The expected name comes from two questions, asked in this order: does another module in the run import it, and if not,
does `__all__` list it?

```mermaid
flowchart TD
    imported{"Does another module in the run import it?"}
    imported -->|yes| keep["Keep its leading underscore as it is,<br/>upper-case the rest"]
    imported -->|no| listed{"Is it listed in #95;#95;all#95;#95;?"}
    listed -->|yes| public["UPPER_CASE"]
    listed -->|no| private["_UPPER_CASE"]
    classDef blue fill:#3776ab,stroke:#062d59,color:#ffffff
    classDef yellow fill:#ffd43b,stroke:#b8930b,color:#062d59
    classDef green fill:#22c55e,stroke:#15803d,color:#052e16
    classDef teal fill:#14b8a6,stroke:#0f766e,color:#042f2e
    classDef orange fill:#f97316,stroke:#c2410c,color:#431407
    class imported,listed yellow
    class keep blue
    class public green
    class private teal
```

## Example

```python
limit = 3
logger = logging.getLogger(__name__)

__all__ = ["logger"]
```

Use instead:

```python
_LIMIT = 3
LOGGER = logging.getLogger(__name__)

__all__ = ["LOGGER"]
```

## Fix safety

The fix renames every use in the file, including the `__all__` entry. It is always unsafe: importers outside the run
keep the old name, and so do string references in unchecked files, such as
[`mocker.patch`](https://pytest-mock.readthedocs.io/en/latest/usage.html) targets.

No fix is offered when:

- another module in the run imports the name or reaches it by a dotted string, such as a `mocker.patch` target
- the new name is already used as a name in the file
- the name shadows a builtin, as in `zip = zip`, where the value reads the builtin under the same name
- the old name appears in the text of a string, as written or through an escape such as `\N{LATIN SMALL LETTER X}`,
  other than an entry of an `__all__` list assigned with `=` or `+=`
- the old name also names something a text rename cannot tell apart: a parameter, a local variable, a class attribute, a
  keyword argument or an attribute access

## Options

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python)
