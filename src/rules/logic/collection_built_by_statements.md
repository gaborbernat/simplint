Flags an empty dict, list or set that the statements right after it only fill with entries, which the literal could hold
instead.

## Why is this bad?

Filling a collection one statement at a time spreads one value over several lines, and a reader has to follow each add
to learn what the collection holds. A literal shows the whole value where it is created, and the entries sit in the
order they are added.

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

An assignment of `{}`, `[]` or `set()` to one name is reported when the statements right after it, one after another,
each add an entry to that name: `name[key] = value` for a dict, `name.append(value)` for a list and `name.add(value)`
for a set. The run stops at the first statement that is not such an add.

Keys and values must be constants or plain names, and must not read the collection itself.

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

- **A set filled with constants**: a set display takes the constants in the order given.

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

- **An entry that calls something or reads an attribute or a subscript**: the call runs in its statement's place, and
  moving it into the literal would reorder it against the other entries and the collection's creation.

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

- **Two keys, or two set elements, that are the same**: the second add replaces the first, which a literal would show as
  a repeated key that Ruff reports. Values that hash alike count as the same, such as `1` and `True`, and so do two
  mentions of one name.

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

- **Two or more appends to a list that run one after another**:
  [FURB113](https://docs.astral.sh/ruff/rules/repeated-append/) fixes them, so this rule leaves them to Ruff. With an
  append to another list or a comment between them, Ruff reports without a fix, and this rule keeps its fix for the
  first entry.

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

The fix is safe when every key and value is a constant. Building such a literal cannot raise, so it stops at the same
point as the statements. A name may be undefined or hold an unhashable value, and then the statements would leave the
collection partly filled while the literal leaves it unbound. Those runs are fixed as unsafe.

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

Ruff's [unnecessary-collection-call](https://docs.astral.sh/ruff/rules/unnecessary-collection-call/) (C408) rewrites
`dict()` and `list()` calls into literals, and does not look at the statements that follow them.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of preview rule
  [FURB113](https://docs.astral.sh/ruff/rules/repeated-append/), which merges consecutive `append` calls into `extend`
  and which this rule leaves to it; this rule covers dicts, sets and the appends FURB113 does not fix.
- [flake8-simplify](https://github.com/MartinThoma/flake8-simplify) reports a dictionary filled by the statements after
  it as [SIM904](https://github.com/MartinThoma/flake8-simplify#sim904). It covers dictionaries only, and it does not
  fix them.
- [pylint](https://github.com/pylint-dev/pylint) reports the same dictionaries as
  [dict-init-mutate](https://pylint.readthedocs.io/en/latest/user_guide/messages/convention/dict-init-mutate.html). It
  covers dictionaries only, with no fix.
