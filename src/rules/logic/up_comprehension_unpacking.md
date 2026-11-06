Flags a nested `for` clause that only unpacks its iterable, and moves the unpacking into the element, which Python 3.15
allows.

## Why is this bad?

A second `for` clause that does nothing but unpack the outer item takes a loop to say what the element can say in one
place. [PEP 798](https://peps.python.org/pep-0798/) lets the element unpack with `*` for lists, sets and generators, and
with `**` for dicts.

## Example

```toml config
[project]
requires-python = ">=3.15"
```

```python report
groups = [[1, 2], [3]]
flat = [x for group in groups for x in group]
```

Use instead:

```python ok
groups = [[1, 2], [3]]
flat = [*group for group in groups]
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python ok
  groups = [[1, 2], [3]]
  flat = [x for group in groups for x in group]
  ```

## What it does

Reports a list, set or generator comprehension with exactly two `for` clauses, or a dict comprehension whose inner
clause unpacks `.items()`, when:

- the outer clause binds a name, `group` here, and the inner clause iterates that name, with no `if` clause and no
  `async` in either.
- the element is the inner clause's target, a name other than `group`. For a dict, the key and value are the two names
  of the inner target.

### Active on

The list, set and generator comprehensions, which unpack with `*`:

```toml config
[project]
requires-python = ">=3.15"
```

```python report
groups = [[1, 2], [3]]
flat_set = {x for group in groups for x in group}
flat_gen = (x for group in groups for x in group)
```

```python fixed
groups = [[1, 2], [3]]
flat_set = {*group for group in groups}
flat_gen = (*group for group in groups)
```

The dict comprehensions, which unpack with `**`:

```toml config
[project]
requires-python = ">=3.15"
```

```python report
pairs = [{"a": 1}, {"b": 2}]
merged = {key: value for entry in pairs for key, value in entry.items()}
```

```python fixed
pairs = [{"a": 1}, {"b": 2}]
merged = {**entry for entry in pairs}
```

### Exceptions

- **A comprehension with an `if` clause or `async`**: the clauses no longer read as one unpacking.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  groups = [[1, 2], [3]]
  flat = [x for group in groups for x in group if x]
  ```

- **An inner iterable other than the outer name**: the element comes from another source.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  groups = [[1, 2], [3]]
  other = [[4]]
  flat = [x for group in groups for x in other]
  ```

- **A dict over `.keys()`, or with a call that takes arguments**: the pairs do not come from `.items()` alone.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  pairs = [{"a": 1}, {"b": 2}]
  keys = {key: value for entry in pairs for key, value in entry.keys()}
  ```

- **Target versions before 3.15**: the syntax is a `SyntaxError` there.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python ok
  groups = [[1, 2], [3]]
  flat = [x for group in groups for x in group]
  ```

### Fix

The fix is safe for lists, sets and generators. It rewrites the element and removes the inner `for` clause. The fix for
a dict is unsafe: `**` reads `keys()` and `__getitem__`, not `items()`, so the rewrite needs a real `dict`.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
  [RUF017](https://docs.astral.sh/ruff/rules/quadratic-list-summation/) writes the same unpacking when fixing
  `sum(lists, [])`.
