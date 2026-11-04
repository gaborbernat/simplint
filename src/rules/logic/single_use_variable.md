Flags a local variable that is assigned once and read once, in the statement right after the assignment. Inlining its
value removes the name and keeps the value next to the code that uses it.

## Why is this bad?

A name that exists only to be read on the next line adds a step to follow without adding meaning. The reader has to find
the assignment, remember the value, and then reach the line that uses it. Inlining the value keeps the decision in one
place.

## Example

```python report
def total_price(prices: list[float]) -> str:
    total = sum(prices)
    return f"{total:.2f}"
```

Use instead:

```python ok
def total_price(prices: list[float]) -> str:
    return f"{sum(prices):.2f}"
```

## Configuration

- [`line-length`](https://docs.astral.sh/ruff/settings/#line-length) and
  [`indent-width`](https://docs.astral.sh/ruff/settings/#indent-width), which give the width a statement must fit after
  the value is inlined; see
  [Settings shared with Ruff](https://simplint.readthedocs.io/en/latest/configuration/project.html#settings-shared-with-ruff)

## What it does

### Active on

- **A plain assignment to one name whose only other mention is the next statement's read**: the name is bound once in
  the function, and read once, in the statement directly after the assignment, in the same block.

  ```python report
  def price(prices: list[float]) -> str:
      total = sum(prices)
      return f"{total:.2f}"


  def show(items: list[str]) -> None:
      count = len(items)
      print(count)
  ```

  ```python fixed
  def price(prices: list[float]) -> str:
      return f"{sum(prices):.2f}"


  def show(items: list[str]) -> None:
      print(len(items))
  ```

### Exceptions

- **A name that is `_`, or a dunder such as `__all__`**: `_` is a placeholder, and a dunder has a meaning to Python.

  ```python ok
  def run(items: list[str]) -> None:
      _ = len(items)
      print(_)
  ```

- **A name the function declares [`global`](https://docs.python.org/3/reference/simple_stmts.html#the-global-statement)
  or [`nonlocal`](https://docs.python.org/3/reference/simple_stmts.html#the-nonlocal-statement)**: code outside the
  function reads the binding too.

  ```python ok
  def run(items: list[str]) -> None:
      global count
      count = len(items)
      print(count)
  ```

- **A value that is a `yield`, `await` or `:=` expression**: these run or bind in a way that moving them changes.

  ```python ok
  async def run(source: Source) -> None:
      data = await source.read()
      print(data)
  ```

- **A next statement that is `return name` or `yield name`**: Ruff's
  [unnecessary-assign](https://docs.astral.sh/ruff/rules/unnecessary-assign/) covers the `return` case, and `yield`
  keeps the same shape.

  ```python ok
  def run(items: list[str]) -> int:
      count = len(items)
      return count
  ```

- **A read inside a loop body, a comprehension, a lambda or a nested function**: inlining would compute the value once
  per pass instead of once.

  ```python ok
  def run(rows: list[list[int]]) -> list[int]:
      total = sum(rows[0])
      return [total + value for value in rows[1]]
  ```

- **A read in a nested block of the next statement**, such as the body of an `if`: the value would run only when the
  block does, so inlining would skip it when the block is not taken.

  ```python ok
  def run(items: list[str], verbose: bool) -> None:
      count = len(items)
      if verbose:
          print(count)
  ```

- **An assignment with a comment on its line, or between it and the read**: inlining would lose the comment.

  ```python ok
  def run(items: list[str]) -> None:
      count = len(items)  # the number of rows
      print(count)
  ```

- **A name read in an `if` test that starts with it**: [walrus-candidate](SMP502.html) covers those, and writes the
  value into the test.

  ```python ok
  def run(text: str) -> None:
      found = text.find("x")
      if found:
          print(found)
  ```

- **A name read again elsewhere in the function**, or bound again: the name is needed after the read, or the read is not
  the only one.

  ```python ok
  def run(items: list[str]) -> None:
      count = len(items)
      print(count)
      print(count)
  ```

- **Functions that read their locals by name, such as `locals()` or `eval`**: a removed name would vanish from what they
  see.

  ```python ok
  def run(items: list[str]) -> None:
      count = len(items)
      print(locals())
  ```

- **A next statement that is a `while` loop**: its test runs again on every pass.

  ```python ok
  def run(items: list[str]) -> None:
      limit = len(items)
      while limit:
          limit -= 1
  ```

- **An inlined header that would no longer fit [`line-length`](https://docs.astral.sh/ruff/settings/#line-length)**:
  `ruff format` would split the statement over several lines, which reads worse than the two statements. The header is
  measured joined on one line at its block's indentation, with the value in place of the name.

  ```toml config
  [tool.ruff]
  line-length = 40
  ```

  ```python ok
  def run(items: list[str]) -> None:
      count = len(items) + len(items) + len(items)
      print(count + 1000000)
  ```

- **A message bound for an exception call in a `raise`**: Ruff's
  [raw-string-in-exception](https://docs.astral.sh/ruff/rules/raw-string-in-exception/) and
  [f-string-in-exception](https://docs.astral.sh/ruff/rules/f-string-in-exception/) ask for an exception's message to be
  a variable, so inlining it would bring back the report those rules remove.

  ```python ok
  def run(items: list[str]) -> None:
      msg = f"{len(items)} items"
      raise ValueError(msg)
  ```

- **A name bound another way in the function**, such as by an `import`, a `def`, a `class`, an `except` clause or a
  `match` capture: the other binding is a second assignment the walk does not follow, so the name is not single-use.

  ```python ok
  def run(items: list[str]) -> None:
      count = len(items)
      from os import count

      print(count)
  ```

### Fix

The fix deletes the assignment and writes the value where the name was, in parentheses when its precedence requires
them.

**Unsafe fix:** the next statement calls something, awaits, or reads an attribute or a subscript before the read, so the
value would run after it instead of before.

```python report
def run(items: list[str], log: Log) -> None:
    count = len(items)
    log.write(count)
```

```python fixed
def run(items: list[str], log: Log) -> None:
    log.write(len(items))
```

Here `log.write` is looked up before `len(items)` runs, which differs only when that lookup has side effects.
