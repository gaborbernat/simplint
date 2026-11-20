Flags two nested `for` clauses that Python 3.15 can express with unpacking in a comprehension.

## Why is this bad?

When the inner loop only yields the elements of each group, `[*group for group in groups]` expresses the same iteration
in one clause. [PEP 798](https://peps.python.org/pep-0798/#rationale) also allows `*` in sets and generator expressions
and `**` in dict comprehensions.

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

The rule checks comprehensions and generator expressions with exactly two synchronous `for` clauses and no filters. For
a list, set or generator, the outer clause binds a name, the inner clause iterates that name, and the element is the
inner target. For a dict, the inner clause must unpack `name.items()` into two distinct names and use them in order as
the key and value.

### Active on

Lists, sets and generator expressions unpack each group with `*`.

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

Dict comprehensions unpack each mapping with `**`.

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

The fix removes parentheses around the original key and value.

```toml config
[project]
requires-python = ">=3.15"
```

```python report
pairs = [{"a": 1}, {"b": 2}]
merged = {(key): (value) for entry in pairs for key, value in entry.items()}
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

- **A dict over [`.keys()`](https://docs.python.org/3/builtins/stdtypes.html#dict.keys), or with a call that takes
  arguments**: the pairs do not come from `.items()` alone.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  pairs = [{("a", 1): True}, {("b", 2): True}]
  keys = {key: value for entry in pairs for key, value in entry.keys()}
  ```

- **Target versions before 3.15**: the syntax is a
  [`SyntaxError`](https://docs.python.org/3/builtins/exceptions.html#SyntaxError) there.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python ok
  groups = [[1, 2], [3]]
  flat = [x for group in groups for x in group]
  ```

### Fix

simplint marks list, set and generator rewrites as safe. The dict rewrite is unsafe because `**` uses the mapping
interface. An object can provide `items()` without supporting unpacking, or return different values for key lookups. If
a comment lies in the element or in the removed clause, simplint reports the comprehension without a fix.

```toml config
[project]
requires-python = ">=3.15"
```

```python report
groups = [[1, 2], [3]]
flat = [
    x
    for group in groups  # one list per group
    for x in group
]
```

```python fixed
groups = [[1, 2], [3]]
flat = [
    x
    for group in groups  # one list per group
    for x in group
]
```

## Other tools

- On Python 3.15 and later, Ruff's [RUF017](https://docs.astral.sh/ruff/rules/quadratic-list-summation/) can use the
  same list unpacking syntax to replace [`sum(lists, [])`](https://docs.python.org/3/builtins/functions.html#sum).
