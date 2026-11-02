## What it does

Checks for `+` chains that join a string literal or
[f-string](https://docs.python.org/3/reference/lexical_analysis.html#f-strings) with other values. Chains that contain a
bytes literal are not reported.

## Why is this bad?

A chain of `+` scatters the text across quotes and operators, and every non-string operand needs an explicit `str()`. An
f-string ([PEP 498](https://peps.python.org/pep-0498/)) shows the final text in one piece.

## Example

```python
message = "Hello, " + name + "!"
```

Use instead:

```python
message = f"Hello, {name}!"
```

## Fix safety

The fix is always unsafe. `+` raises [`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError) for a
non-string operand and honors a custom [`__add__`](https://docs.python.org/3/reference/datamodel.html#object.__add__) or
`__radd__`, such as `markupsafe.Markup` escaping the other operand, while an f-string formats any value. So the fix is
offered only when the code shows every operand is a `str`:

- a string literal or f-string, `"..." % values`, or a `str` repeated by an `int`, such as `"-" * width`;
- a call to the builtin `str`, `repr`, `ascii`, `format`, `chr`, `hex`, `oct` or `bin`, or a `str` method such as
  `join`, `format` or `upper` called on a `str`, or `decode` on `bytes`;
- an index or slice of a `str`, or an item of a list, tuple, set or mapping of them, such as `name.split(".")[0]` or
  `names[0]` for `names: list[str]`;
- a conditional between `str` values, or `a or "default"` where `a` is a `str` or `None`;
- `__name__`, `__qualname__` and `__module__` of anything, and the module's `__name__` and `__file__`;
- standard library values the module imports: `os.sep`, `os.linesep`, `sys.platform`, the `string` constants,
  `os.getcwd()`, `platform.system()`, `json.dumps(...)`, `textwrap.dedent(...)`, `os.environ[...]`,
  `os.environ.get(key, "default")`, `os.getenv(key, "default")`, `uuid.uuid4().hex`, `os.path.join(...)` and the other
  `os.path` functions given a `str`, and the `name`, `stem`, `suffix` and `as_posix()` of a `pathlib.Path`;
- a call to a function or method of the module annotated `-> str`, undecorated or with `functools.cache`;
- an attribute of an instance of a class of the module that annotates it `str`, or a property annotated `-> str`; on
  `self`, also one that the class body and its methods only ever set to such values, when nothing else writes it and no
  subclass may override it;
- a name every binding of which, in the scope it resolves to, gives such a value: an assignment, a loop over such items,
  a `with tempfile.TemporaryDirectory() as path`, or an annotation `str` (also `Final[str]`, `Annotated[str, ...]`) that
  no binding contradicts.

A name that an unannotated parameter, an import or a `global` binds is not known, so `value + "!"` for an unannotated
`value` is reported without a fix. The annotations are trusted: a parameter annotated `str` can still receive a `str`
subclass with its own `__add__`.

The fix copies each non-literal operand into a placeholder as written. A call to the builtin `str`, `repr` or `ascii`
becomes the matching conversion, as Ruff's
[RUF010](https://docs.astral.sh/ruff/rules/explicit-f-string-type-conversion/) asks, so `"n=" + str(n)` becomes
`f"n={n!s}"`. A chain of plain string literals becomes one plain string.

No fix is offered when:

- a literal is implicitly concatenated, triple-quoted, raw or `u`-prefixed;
- the literals use different quote characters;
- a comment sits between the operands, since the f-string has no place for it;
- a non-literal operand spans several lines or contains a backslash, `#`, `:`, `!`, `{` or `}`;
- the target Python version is older than 3.12 and a non-literal operand contains the string's quote character, which
  f-strings allow only from 3.12 on ([PEP 701](https://peps.python.org/pep-0701/#considerations-of-quote-reuse)).

## Options

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python)
