Flags a local variable that is assigned once and read once, in the statement right after the assignment. Inlining its
value removes the name and keeps the value next to the code that uses it.

## Why is this bad?

When the name adds no meaning, inlining the value lets the reader understand the next statement in one place.
[Fowler's Extract Variable](https://refactoring.com/catalog/extractVariable.html) gives a name to part of an expression,
and [Sourcery](https://docs.sourcery.ai/reference/refactorings/python/inline-variable/) warns that inlining can hurt
readability. A name that explains and one that adds nothing look the same to the rule.

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
  them is inlined into a default; see the parameter-default exception below

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

- **A raw f-string whose debug text holds its quote**: a raw f-string keeps the backslash that an escaped quote would
  need, so the debug text cannot be written in it. The read is reported, but no fix is offered.

  ```toml config
  [tool.ruff]
  target-version = "py312"
  ```

  ```python report
  def show(items: list[str]) -> None:
      total = len(items)
      print(rf"{"k" * total=}")
  ```

  ```python fixed
  def show(items: list[str]) -> None:
      total = len(items)
      print(rf"{"k" * total=}")
  ```

- **A string or f-string value inlined into a bare field of an f-string**: its text joins the enclosing f-string, so
  `f"[{inner}]"` with `inner = f"<{value}>"` becomes `f"[<{value}>]"` rather than a nested f-string.

  ```python report
  def wrap(value: str) -> str:
      inner = f"<{value}>"
      return f"[{inner}]"
  ```

  ```python fixed
  def wrap(value: str) -> str:
      return f"[<{value}>]"
  ```

- **Before Python 3.12, a value whose strings use the f-string's quote**: a string in the value takes the other quote,
  which the field can hold. A value spread over lines goes in on one line, so it fits inside a single-quoted or
  double-quoted f-string.

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

- **A read as the argument of [`next`](https://docs.python.org/3/builtins/functions.html#next) or
  [`anext`](https://docs.python.org/3/builtins/functions.html#anext), or as the receiver of `send`, `throw`, `close` or
  `__next__`**: the call steps or closes the object the name holds. Without the name, the call holds the last reference
  to the object, so the object is released when the call returns, which runs a generator's `finally` early. Keeping the
  name keeps the object alive, so the variable is not reported.

  ```python ok
  def run(rows: list[int]) -> None:
      stream = produce(rows)
      print(next(stream))
  ```

- **A weak reference**: a call to [`weakref.ref`](https://docs.python.org/3/library/weakref.html#weakref.ref),
  [`weakref.proxy`](https://docs.python.org/3/library/weakref.html#weakref.proxy) or
  [`weakref.WeakMethod`](https://docs.python.org/3/library/weakref.html#weakref.WeakMethod). Inlined, the weak reference
  is dropped right after the read, so code that counts or relies on it, as
  [`weakref.getweakrefcount`](https://docs.python.org/3/library/weakref.html#weakref.getweakrefcount) does, sees it
  gone.

  ```python ok
  import weakref


  def count(node: object) -> int:
      reference = weakref.ref(node)
      print(reference())
      return weakref.getweakrefcount(node)
  ```

- **A next statement that is `return name` in a function that does not yield**: Ruff's
  [unnecessary-assign](https://docs.astral.sh/ruff/rules/unnecessary-assign/) (RET504) covers it, so this rule leaves it
  to Ruff whether or not the project selects that rule. RET504 skips a function that yields, so a generator's return is
  inlined here.

  ```python ok
  def run(items: list[str]) -> int:
      count = len(items)
      return count
  ```

  ```python report
  def run(items: list[str]) -> Iterator[int]:
      yield 0
      count = len(items)
      return count
  ```

  ```python fixed
  def run(items: list[str]) -> Iterator[int]:
      yield 0
      return len(items)
  ```

- **A next statement that is `yield name`**: never reported, since the value is yielded, not returned.

  ```python ok
  def run(items: list[str]) -> Iterator[int]:
      count = len(items)
      yield count
  ```

- **A next statement that is `yield from name`**: Ruff's preview rule
  [unnecessary-assign-before-yield](https://docs.astral.sh/ruff/rules/unnecessary-assign-before-yield/) (RUF070) covers
  it, so this rule leaves the pair to Ruff whether or not the project selects that rule or enables preview.

  ```python ok
  def run(items: list[str]) -> Iterator[int]:
      lengths = [len(item) for item in items]
      yield from lengths
  ```

- **A read in a parameter's default, while Ruff's
  [mutable-argument-default](https://docs.astral.sh/ruff/rules/mutable-argument-default/) (B006) or
  [function-call-in-default-argument](https://docs.astral.sh/ruff/rules/function-call-in-default-argument/) (B008)
  runs**: the default is evaluated once, when the function is defined, and inlining a value that one of them reports
  would make Ruff report it, so the variable stays. B006 reports a mutable display, a comprehension, or a call of a
  mutable type such as `list()` or
  [`collections.Counter()`](https://docs.python.org/3/library/collections.html#collections.Counter). B008 reports a
  call, except one of an immutable type such as `tuple()`, a
  [`NewType`](https://docs.python.org/3/library/typing.html#newtype) of an immutable type, or a callee listed in
  `extend-immutable-calls` (see Configuration). A call inside a lambda is not reported. A parameter annotated with an
  immutable type is not reported by either, so the value is still inlined. The inline is skipped whenever Ruff runs the
  rule, even where `per-file-ignores` or `noqa` silence it. The variable is reported when the project's Ruff
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

- **A list comprehension used by [`all()`](https://docs.python.org/3/builtins/functions.html#all) or
  [`any()`](https://docs.python.org/3/builtins/functions.html#any)** when it contains a call, lookup or other possible
  side effect. Inlining would expose the comprehension to Ruff's
  [C419](https://docs.astral.sh/ruff/rules/unnecessary-comprehension-in-call/), whose generator fix can skip later
  elements.

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

- **A read in an annotation, a `type` statement or a type parameter**: local variable annotations inside functions are
  unevaluated. Other annotations can run at definition time or later, depending on the target Python version. Inlining
  here could change when the value is computed.

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

- **Before Python 3.12, a value whose strings hold the quote of any f-string around the field**: the inlined text joins
  the field's own f-string and each one that encloses it, and Python before 3.12 rejects a quote that an enclosing
  f-string already uses inside a replacement field. From 3.12 on, the same value is inlined.

  ```toml config
  [tool.ruff]
  target-version = "py311"
  ```

  ```python ok
  def quoted(suffix: str) -> str:
      label = 'say "hi"'
      return f"{f'{label}'}" + suffix
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

- **A name read or bound elsewhere in the function**, including a reference in a quoted type annotation or `ForwardRef`.

  ```python ok
  def run(items: list[str]) -> None:
      count = len(items)
      print(count)
      print(count)
  ```

- **Functions that read their locals by name, such as
  [`locals()`](https://docs.python.org/3/builtins/functions.html#locals) or `eval`**: a removed name would vanish from
  what they see.

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

**Unsafe fix.** In `size = limit` followed by `log.write(size)`, Python reads `limit` before looking up `log.write`.
Inlining changes that order. The rule marks the fix unsafe because a lookup can run user code that rebinds the name.

```python report
def run(limit: int, log: Log) -> None:
    size = limit
    log.write(size)
```

```python fixed
def run(limit: int, log: Log) -> None:
    log.write(limit)
```

**No fix.** Simplint keeps the assignment when moving its value could reorder side effects or shorten the lifetime of a
temporary object. It checks for these cases.

- A call, `await`, attribute access or subscript would run before an operation it previously followed.
- The next statement observes a value the assignment might change. This includes formatting a name in an f-string,
  unpacking it with `*` or `**`, using it in an operator or augmented assignment, and reading a name that another
  function declares `global` or `nonlocal`.
- The assigned value contains a call, and the next statement extracts an attribute or item from the result. Inlining can
  release the object immediately after the lookup and run its finalizer earlier.
- A `:=` inside the assigned value would rebind a name after the next statement had already read it.

```python report
def evaluate(stack: list[str]) -> float:
    right = operand(stack)
    left = operand(stack)
    return left / right
```

Inlining `left` keeps the two calls in order. Also inlining `right` would produce `operand(stack) / operand(stack)`,
which pops the numerator first. The original pops the denominator first.

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

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [RET504](https://docs.astral.sh/ruff/rules/unnecessary-assign/) and preview rule
  [RUF070](https://docs.astral.sh/ruff/rules/unnecessary-assign-before-yield/), for assignments before `return` and
  `yield from`. A broader single-use check is tracked in [#10108](https://github.com/astral-sh/ruff/issues/10108).
- Sourcery's [inline-variable](https://docs.sourcery.ai/reference/refactorings/python/inline-variable/) inlines a single
  use when doing so enables a further refactoring, such as replacing a loop with a comprehension. Simplint requires the
  read to occur in the next statement.
