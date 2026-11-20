Flags an empty dict, list or set followed by statements that could put the same entries in its literal.

## Why is this bad?

A literal shows the initial contents in one place. With separate assignments or method calls, the reader has to follow
mutations to see what the collection contains.

## Example

```python report
def headers(token: str) -> dict[str, str]:
    headers = {}
    headers["Accept"] = "application/json"
    headers["Cache"] = "no-store"
    return headers
```

```python fixed
def headers(token: str) -> dict[str, str]:
    headers = {"Accept": "application/json", "Cache": "no-store"}
    return headers
```

## What it does

### Active on

The rule starts at `name = {}`, `name = []` or `name = set()`. It collects consecutive `name[key] = value`,
`name.append(value)` or `name.add(value)` statements, depending on the collection. Keys and values must be constants or
plain names. The scan stops at the first statement that fails these checks.

- **A list filled with an append**: the entry is written in the literal. A run of two or more appends is left to Ruff's
  FURB113, under Exceptions.

  ```python report
  def names(first: str) -> list[str]:
      out = []
      out.append(first)
      return out
  ```

  ```python fixed
  def names(first: str) -> list[str]:
      out = [first]
      return out
  ```

- **A set filled with constants.** The fix puts the elements in a set literal.

  ```python report
  def unique() -> set[int]:
      seen = set()
      seen.add(1)
      seen.add(2)
      return seen
  ```

  ```python fixed
  def unique() -> set[int]:
      seen = {1, 2}
      return seen
  ```

### Exceptions

- **An entry that calls a function, looks up an attribute or indexes an object.** Moving an operation into the literal
  would run it before the assignment binds the collection's name.

  ```python ok
  def build() -> dict[str, int]:
      table = {}
      table["k"] = compute()
      table["j"] = 1
      return table


  def compute() -> int:
      return 1
  ```

- **An entry that reads the collection**: the literal does not exist yet while its entries are built.

  ```python ok
  def build(name: str) -> dict[str, object]:
      table = {}
      table["self"] = table
      table[name] = 1
      return table
  ```

- **A statement between the literal and its first add**: the run is empty, so nothing is reported.

  ```python ok
  def build(log: list[str]) -> dict[str, int]:
      table = {}
      log.append("start")
      table["b"] = 2
      return table
  ```

- **Duplicate keys or set elements.** For example, Python treats `1` and `True` as the same key. The rule also skips a
  dict or set whose literal would contain the same name twice.

  ```python ok
  def build() -> dict[str, int]:
      table = {}
      table["k"] = 1
      table["k"] = 2
      return table
  ```

  ```python ok
  def build() -> set[object]:
      seen = set()
      seen.add(1)
      seen.add(True)
      return seen
  ```

- **An annotated assignment**: the literal would need the annotation, which the rule does not rewrite.

  ```python ok
  def build() -> dict[str, int]:
      table: dict[str, int] = {}
      table["k"] = 1
      return table
  ```

- **A comment between two adds**: the comment would lose the statement it stands beside.

  ```python ok
  def build() -> dict[str, int]:
      table = {}
      table["a"] = 1
      # the second key
      table["b"] = 2
      return table
  ```

- **Consecutive appends to one list.** The rule leaves these to Ruff's
  [repeated-append (FURB113)](https://docs.astral.sh/ruff/rules/repeated-append/). If an append to another list
  interrupts the sequence, simplint can still fold the first list's initial entries into its literal. A comment inside
  the span simplint would replace prevents the rewrite.

  ```python ok
  def build() -> list[int]:
      result = []
      result.append(1)
      result.append(2)
      return result
  ```

### Fix

The fix replaces the run with one literal of the same kind and keeps the assignment's name. A tuple used as a key or an
entry is parenthesized, since a comma would end it.

The fix is marked safe when all entries are constant. A name lookup can fail, and a dict key or set element can be
unhashable. With separate statements, the name already refers to the empty or partly filled collection when that
happens. With a literal, the assignment never takes place. Fixes involving names are marked unsafe.

```python report
def keyed(name: str, value: int) -> dict[str, int]:
    result = {}
    result[name] = value
    return result
```

```python fixed
def keyed(name: str, value: int) -> dict[str, int]:
    result = {name: value}
    return result
```

## Ruff overlap

Ruff's [FURB113](https://docs.astral.sh/ruff/rules/repeated-append/) can replace consecutive list appends with `extend`.
Its [C408](https://docs.astral.sh/ruff/rules/unnecessary-collection-call/) replaces calls such as `dict()` and `list()`
with empty literals.

## Other tools

- [flake8-simplify's SIM904](https://github.com/MartinThoma/flake8-simplify#sim904) reports assignments that could be
  included in a dict literal.

- Pylint's
  [dict-init-mutate (C3401)](https://pylint.readthedocs.io/en/latest/user_guide/messages/convention/dict-init-mutate.html)
  also reports assignments to a newly created dict.
