## What it does

Checks for [dataclasses](https://docs.python.org/3/library/dataclasses.html#dataclasses.dataclass) declared without
`kw_only=True`. The decorator must be spelled `@dataclass` or `@dataclasses.dataclass`. Any value other than a literal
`True`, such as `kw_only=False`, is reported too.

## Why is this bad?

Positional construction ties every caller to the field order, so reordering or inserting a field silently shifts values
into the wrong fields. Keyword-only fields name each value at the call site. The `kw_only` option needs Python 3.10 or
later, so nothing is reported when the target version is older.

In the other files simplint checks, the rule also reports each call that passes such a dataclass's fields by position,
so the fix can rewrite it.

[SMP111](SMP111.html) asks for `frozen=True` and `slots=True` on the same decorator.

## Example

```python
@dataclass(frozen=True, slots=True)
class Release:
    name: str
    version: str


Release("simplint", "1.0.0")
```

Use instead:

```python
@dataclass(frozen=True, slots=True, kw_only=True)
class Release:
    name: str
    version: str


Release(name="simplint", version="1.0.0")
```

## Fix safety

The fix sets `kw_only=True` on the decorator and passes every positional argument of the calls in the same module by
keyword, using the field order of the generated `__init__`: inherited fields first, without `ClassVar` fields, fields
with `init=False` or `kw_only=True` and the fields after `KW_ONLY`, and with `InitVar` fields. Calls through `cls(...)`
in the class's own methods are rewritten too. A call that unpacks `**kwargs` passes those fields by keyword, so it keeps
working. In another file, each positional call gets its own report, whose fix passes the arguments by keyword. That fix
is safe, as the call does the same before and after.

The fix of the decorator counts on those reports: the same `--fix` run rewrites the calls of the other files and adds
`kw_only=True`. No fix is offered while any use could still pass fields by position:

- a call that unpacks `*args`, or passes more arguments than the class has positional fields;
- the class used as a value, as in `functools.partial(Partition, topic)`, or matched by position in a `case` pattern;
- a subclass, or a call through `type(self)(...)` or `self.__class__(...)`;
- a file that imports the module with `*`, or that imports the class and passes it on to other modules;
- a positional call simplint cannot tie to the class, such as through a local name that shadows the import, or of an
  attribute with the class's name on something no import binds, such as `module.Partition("a", 1)` on a module a test
  loads from its file path;
- a positional call in a file whose own report cannot rewrite it: one that ignores `dataclass-kw-only`, does not have it
  in `fixable`, targets a Python version before 3.10, or has a suppression comment on the call;
- a class nested in a function or another class, one that defines `__init__` or `__new__`, sets `init=False`, or has a
  base class that is not a dataclass of the same module.

The fix stays unsafe: a caller's fix that simplint withdraws because it gives Ruff work it cannot fix, and code outside
the checked files, such as the users of a published library, can still call the class by position.

## Options

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python)
