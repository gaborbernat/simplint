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
  def lines(rows: list[bytes]) -> str:
      joined = b"\n".join(rows)
      return f"{joined}"
  ```

- [`extend-immutable-calls`](https://docs.astral.sh/ruff/settings/#lint_flake8-bugbear_extend-immutable-calls) in
  `[tool.ruff.lint.flake8-bugbear]`, which adds calls to the ones B008 takes as immutable, so a value calling one of
  them is inlined into a default; see the read in a parameter's default above

  ```toml config
  [tool.ruff.lint]
  select = ["B008"]

  [tool.ruff.lint.flake8-bugbear]
  extend-immutable-calls = ["frozen.freeze"]
  ```

  ```python report
  from frozen import freeze


  def run(items: list[str]) -> None:
      frozen = freeze(items)

      def show(held=frozen) -> None:
          print(held)
  ```

  ```python fixed
  from frozen import freeze


  def run(items: list[str]) -> None:

      def show(held=freeze(items)) -> None:
          print(held)
  ```

- [`preview`](https://docs.astral.sh/ruff/settings/#preview) in `[tool.ruff]`, which makes B006 also report a generator
  and a tuple that holds a mutable value, so such a value is not inlined into a default either

  ```toml config
  [tool.ruff]
  preview = true

  [tool.ruff.lint]
  select = ["B006"]
  ```

  ```python ok
  def run(items: list[str]) -> None:
      rows = (item for item in items)

      def show(held=rows) -> None:
          print(held)

      show()
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

- **A string or f-string value inlined into a bare field of an f-string**: its text joins the enclosing f-string, so
  `f"[{inner}]"` with `inner = f"<{value}>"` becomes `f"[<{value}>]"` rather than a nested f-string.

- **Before Python 3.12, a value whose strings use the f-string's quote**: a string in the value takes the other quote,
  which the field can hold. A value spread over lines goes in on one line, as `ruff format` joins it, since the field
  cannot hold a line break either.

  ```python report
  def wrap(value: str) -> str:
      inner = f"<{value}>"
      return f"[{inner!r}]"
  ```

  ```python fixed
  def wrap(value: str) -> str:
      return f"[{f'<{value}>'!r}]"
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

- **A read as the argument of [`next`](https://docs.python.org/3/library/functions.html#next) or
  [`anext`](https://docs.python.org/3/library/functions.html#anext), or as the receiver of `send`, `throw`, `close` or
  `__next__`**: the call steps or closes the object the name holds. Without the name, the call would be the only holder
  of the object, so the object would be released as soon as the call returns, which runs a generator's `finally` early.
  Keeping the name is the only way to leave the object alone, so the variable is not reported.

  ```python ok
  def run(rows: list[int]) -> None:
      stream = produce(rows)
      print(next(stream))
  ```

- **A next statement that is `return name` or `yield name`**: Ruff's
  [unnecessary-assign](https://docs.astral.sh/ruff/rules/unnecessary-assign/) covers the `return` case while the
  project's Ruff configuration selects it, and `yield` keeps the same shape. The inline is skipped whenever Ruff selects
  the rule, even where `per-file-ignores` or `noqa` silence it.

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
  the project's Ruff configuration selects it, even when this rule's fix would differ. With preview off, or without
  RUF070, this rule reports the pair. The inline is skipped whenever Ruff selects the rule, even where
  `per-file-ignores` or `noqa` silence it.

- **A read in a parameter's default, while Ruff's
  [mutable-argument-default](https://docs.astral.sh/ruff/rules/mutable-argument-default/) (B006) or
  [function-call-in-default-argument](https://docs.astral.sh/ruff/rules/function-call-in-default-argument/) (B008)
  runs**: the default is evaluated once, when the function is defined, and inlining a value that one of them reports
  would make Ruff report it, so the variable stays. B006 reports a mutable display, a comprehension, or a call of a
  mutable type such as `list()` or `collections.Counter()`. B008 reports a call, except one of an immutable type such as
  `tuple()`, a [`NewType`](https://docs.python.org/3/library/typing.html#newtype) of an immutable type, or a callee
  listed in `extend-immutable-calls` (see Configuration). A call inside a lambda is not reported. A parameter annotated
  with an immutable type is not reported by either, so the value is still inlined. The inline is skipped whenever Ruff
  runs the rule, even where `per-file-ignores` or `noqa` silence it. The variable is reported when the project's Ruff
  configuration runs neither rule.

  ```toml config
  [tool.ruff.lint]
  select = ["B006", "B008"]
  ```

  ```python ok
  def run(items: list[str]) -> None:
      count = len(items)

      def show(size=count) -> None:
          print(size)

      show()
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

- **A conditional expression, or a value with a string literal in it, read inside an f-string field**: the field would
  hold the conditional or the literal inside its braces, which reads worse than the two statements. A value that is a
  string or an f-string on its own is still inlined.

  ```python ok
  def run(items: list[str], flag: bool) -> str:
      sep = " " if flag else ""
      return f"[{sep}{items[0]}]"
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

The fix is also unsafe when the value calls or looks something up and the next statement reads a name before the read
that sees what the call changes: a name formatted in an f-string field or unpacked with `*` or `**`, which takes the
object's state at that point, or a name some function in the module declares `global` or `nonlocal`, which the call can
rebind.

```python report
def run(stack: list[int]) -> str:
    top = stack.pop()
    return f"{stack} after popping {top}"
```

```python fixed
def run(stack: list[int]) -> str:
    return f"{stack} after popping {stack.pop()}"
```

Inlined, `stack` is formatted before `stack.pop()` runs, so the message shows the popped item still in the list.

The fix is also unsafe when the value calls something and the next statement reads an attribute or a subscript of the
name, such as `tmp.name`, because the call's object is dropped right after that read, so its finalizer runs early. A
method call on the name, such as `tmp.describe()`, keeps the object alive while it runs, so that fix stays safe.

```python report
import tempfile
from pathlib import Path


def make_note() -> None:
    tmp = tempfile.TemporaryDirectory()
    Path(tmp.name, "note").write_text("hi")
```

```python fixed
import tempfile
from pathlib import Path


def make_note() -> None:
    Path(tempfile.TemporaryDirectory().name, "note").write_text("hi")
```

The fix is also unsafe when the value calls something and the next statement reads a name in an operator before the
read, such as `not queue`, because the operator takes the name's state before the call runs.

```python report
def pop_once(queue: list[int]) -> tuple[bool, int]:
    head = queue.pop()
    return (not queue, head)
```

```python fixed
def pop_once(queue: list[int]) -> tuple[bool, int]:
    return (not queue, queue.pop())
```

An augmented assignment before the read is unsafe when the value calls something, because the target is loaded before
that call runs, or when its target is an attribute or a subscript, whose object and lookup run first.

```python report
def run(counter: Counter) -> int:
    step = counter.bump()
    counter.total += step
    return counter.total
```

```python fixed
def run(counter: Counter) -> int:
    counter.total += counter.bump()
    return counter.total
```

```python report
def count_items(items: list[int]) -> int:
    total = 0
    step = len(items)
    total += step
    return total
```

```python fixed
def count_items(items: list[int]) -> int:
    total = 0
    total += len(items)
    return total
```

The fix is also unsafe when the value holds a walrus that rebinds a name the next statement reads before the read,
because that read would see the new value.

```python report
def run(reader: Reader) -> tuple[int, int]:
    y = 0
    count = (y := reader.read()) + 1
    return (y, count)
```

```python fixed
def run(reader: Reader) -> tuple[int, int]:
    y = 0
    return (y, ((y := reader.read()) + 1))
```

A read in a t-string is reported without a fix, because a t-string keeps the source text of each field, which the
inlined value would change.

```toml config
[tool.ruff]
target-version = "py314"
```

```python report
def render(a: int) -> object:
    value = a + 1
    return t"{value}"
```

```python fixed
def render(a: int) -> object:
    value = a + 1
    return t"{value}"
```

A string value with braces that replaces a field in a format spec, as in `{value:{inner}}`, is written as an expression
inside the nested field instead of being spliced into the f-string, because `{` and `}` are not escapes in a format
spec.

```toml config
[tool.ruff]
target-version = "py311"
```

```python report
def render(value: object) -> str:
    inner = "a{b}"
    return f"{value:{inner}}"
```

```python fixed
def render(value: object) -> str:
    return f"{value:{'a{b}'}}"
```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rules
  [RET504](https://docs.astral.sh/ruff/rules/unnecessary-assign/) and preview
  [RUF070](https://docs.astral.sh/ruff/rules/unnecessary-assign-before-yield/), which cover a value returned or yielded
  on the next line and which this rule leaves to them; every other single read is open issue
  [#10108](https://github.com/astral-sh/ruff/issues/10108).
- [Sourcery](https://sourcery.ai) inlines a variable that is used once anywhere in its block through
  [inline-variable](https://docs.sourcery.ai/reference/refactorings/python/inline-variable/). This rule only inlines a
  value read on the line right below its assignment.
