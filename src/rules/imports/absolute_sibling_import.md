## What it does

Checks for `from ... import` statements that name the file's own package, or a package below it, by its absolute path. A
file belongs to a package when its directory holds an
[`__init__.py`](https://docs.python.org/3/reference/import.html#regular-packages); simplint builds the package name from
the chain of directories with an `__init__.py` above the file. Imports from a parent package and plain `import a.b`
statements are not reported.

## Why is this bad?

An absolute import repeats the package name, so renaming or vendoring the package means editing every such import. A
[relative import](https://docs.python.org/3/reference/import.html#package-relative-imports)
([PEP 328](https://peps.python.org/pep-0328/)) shows at a glance that the target lives next to the importing module.

Ruff's [TID252](https://docs.astral.sh/ruff/rules/relative-imports/) bans only relative imports from parent packages by
default, which fits this rule. Setting its
[`ban-relative-imports`](https://docs.astral.sh/ruff/settings/#lint_flake8-tidy-imports_ban-relative-imports) option to
`"all"` contradicts it.

## Example

```python
# in package/sub/module.py
from package.sub import sibling
from package.sub.deep import thing
```

Use instead:

```python
# in package/sub/module.py
from . import sibling
from .deep import thing
```

## Fix safety

The fix is safe unless the module has a top-level
[`if __name__ == "__main__":`](https://docs.python.org/3/library/__main__.html#name-main) block. The relative import
resolves to the same module when the file is imported as part of its package. A file run directly with
`python path/to/module.py` has no package, and relative imports fail there.

## Conflicts with Ruff

Ruff's [relative-imports (TID252)](https://docs.astral.sh/ruff/rules/relative-imports/) with
[`ban-relative-imports`](https://docs.astral.sh/ruff/settings/#lint_flake8-tidy-imports_ban-relative-imports) set to
`"all"` asks for the absolute import this rule replaces, and its fix turns the relative import back. simplint then
offers no fix and keeps the report; turn off one of the two rules.
