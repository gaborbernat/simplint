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

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python),
  which decide the f-strings that cannot hold a backslash or their own quote before Python 3.12, and the decorators that
  must be dotted names before Python 3.9; see
  [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [tool.ruff]
  target-version = "py312"
  ```

  ```python report
  def quote(value: str) -> str:
      quoted = value.replace("'", "''")
      return f"'{quoted}'"
  ```

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

- **A dict or set display inlined right after an f-string field's `{`**: the value gets a space inside the braces,
  because `{{` would be read as an escaped brace.

  ```python report
  def show(key: str) -> None:
      mapping = {key: 2}
      print(f"{mapping}")
  ```

  ```python fixed
  def show(key: str) -> None:
      print(f"{ {key: 2} }")
  ```

- **A read in an f-string field with a [`=`](https://docs.python.org/3/reference/lexical_analysis.html#f-strings) debug
  specifier**, such as `{total=}`: the field prints the text of its expression, so the text stays as written and the
  value moves into a plain field after it. The printed text does not change.

  ```python report
  def show(items: list[str]) -> None:
      total = len(items)
      print(f"{total=}")
  ```

  ```python fixed
  def show(items: list[str]) -> None:
      print(f"total={len(items)!r}")
  ```

- **Before Python 3.12, a value whose strings use the f-string's quote**: a string in the value takes the other quote,
  which the field can hold. A value spread over lines goes in on one line, as `ruff format` joins it, since the field
  cannot hold a line break either.

  ```python report
  def join(items: list[str]) -> str:
      sep = " ".join(items)
      return f"[{sep}]"
  ```

  ```python fixed
  def join(items: list[str]) -> str:
      return f"[{' '.join(items)}]"
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
  [unnecessary-assign](https://docs.astral.sh/ruff/rules/unnecessary-assign/) covers the `return` case while the
  project's Ruff configuration selects it, and `yield` keeps the same shape. Ruff's own suppressions of it, through
  `per-file-ignores` or `noqa`, are respected too.

  ```toml config
  [tool.ruff.lint]
  select = ["RET504"]
  ```

  ```python ok
  def run(items: list[str]) -> int:
      count = len(items)
      return count
  ```

- **A next statement that is `yield from name`, while Ruff's preview mode is on**: Ruff's preview rule
  [unnecessary-assign-before-yield](https://docs.astral.sh/ruff/rules/unnecessary-assign-before-yield/) covers it when
  the value goes in as written. With preview off, or for a value that Ruff would write differently, such as a tuple, the
  report stays. The pair is left to Ruff only when the project's Ruff configuration selects the rule. Ruff's own
  suppressions of it, through `per-file-ignores` or `noqa`, are respected too.

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

- **A read after `and` or `or`**, or in a branch of a conditional expression, a later link of a chained comparison, an
  `assert`, a `case` or the type of an `except` clause: each runs only when the code before it does not decide the
  outcome, or when an exception or a match reaches it.

  ```python ok
  def run(items: list[str], flag: bool) -> None:
      count = len(items)
      print(flag and count)
  ```

  ```python ok
  def run(items: list[str], flag: bool) -> None:
      count = len(items)
      print(count if flag else 0)
  ```

  ```python ok
  def run(items: list[str], low: int) -> None:
      count = len(items)
      print(low < 0 < count)
  ```

  ```python ok
  def run(items: list[str]) -> None:
      count = len(items)
      assert count, "no items"
  ```

  ```python ok
  def run(command: str, items: list[str]) -> None:
      count = len(items)
      match command:
          case "count":
              print(count)
  ```

  ```python ok
  def run(items: list[str]) -> None:
      error = ValueError
      try:
          print(items[0])
      except error:
          print("empty")
  ```

- **A read in an annotation, a `type` statement or a type parameter**: an annotation in a function body is never
  evaluated, and the others are evaluated lazily, so the value would not run where the name was read.

  ```python ok
  def run(items: list[str]) -> None:
      kind = list[str]
      values: kind = items
      print(values)
  ```

- **Before Python 3.12, a value with a backslash, or whose strings hold the other quote**: a replacement field cannot
  hold a backslash or the f-string's own quote until [PEP 701](https://peps.python.org/pep-0701/), and a string that
  holds the other quote cannot take it, so the inlined code would not parse.

  ```toml config
  [tool.ruff]
  target-version = "py311"
  ```

  ```python ok
  def quote(value: str) -> str:
      quoted = value.replace("'", "''")
      return f"'{quoted}'"
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

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** released rules
  [RET504](https://docs.astral.sh/ruff/rules/unnecessary-assign/) and
  [RUF070](https://docs.astral.sh/ruff/rules/unnecessary-assign-before-yield/) (preview).
- [Sourcery](https://sourcery.ai) inlines a variable that is used once anywhere in its block through
  [inline-variable](https://docs.sourcery.ai/reference/refactorings/python/inline-variable/). This rule only inlines a
  value read on the line right below its assignment.
