Flags a local constant assignment when at least one statement separates it from its first use in the same block.

## Why is this bad?

Assigning a constant just before its first use lets the reader see the value where it matters.

[Sourcery's move-assign refactoring](https://docs.sourcery.ai/reference/refactorings/python/move-assign/) keeps a
variable close to its uses, which also makes it easier to extract a function.
[Google's C++ style guide](https://google.github.io/styleguide/cppguide.html#Local_Variables) asks for the same,
declaring each variable in the narrowest scope and close to its first use.

## Example

```python report
def total(items: list[int]) -> int:
    count = 0
    items.sort()
    print("sorted")
    return count + len(items)
```

Use instead:

```python ok
def total(items: list[int]) -> int:
    items.sort()
    print("sorted")
    count = 0
    return count + len(items)
```

## What it does

A constant is a number, string, bytes, `True`, `False`, `None` or `...`, or a list, tuple, set or dict display of
constants. The rule also accepts `not` on a constant and unary `+` or `-` on a numeric literal. Dict keys and set
elements must be hashable.

### Active on

A plain assignment of a constant to one name is reported when the first statement that mentions the name reads it, and
that statement comes at least two statements below the assignment in the same block.

- **A constant read further down the function**: the statements between do not touch the name.

  ```python report
  def price(items: list[float]) -> str:
      discount = 0.1
      total = sum(items)
      return f"{total * (1 - discount):.2f}"
  ```

  ```python fixed
  def price(items: list[float]) -> str:
      total = sum(items)
      discount = 0.1
      return f"{total * (1 - discount):.2f}"
  ```

### Exceptions

- **A value that is not a constant**: a name, a call or an operator can read or change state that the statements in
  between also change, so moving it could change its result or when it runs.

  ```python ok
  def total(items: list[int]) -> int:
      base = len(items)
      items.sort()
      return base
  ```

- **A name read by a statement right below the assignment**: that statement is the one
  [single-use-variable](SMP504.html) covers, and there is nothing to move.

  ```python ok
  def total(items: list[int]) -> int:
      count = 0
      return count + len(items)
  ```

- **A first use that binds the name again**, such as a `for` target, an assignment or an augmented assignment: the rule
  only moves the assignment when the first mention reads the name.

  ```python ok
  def last(items: list[int]) -> int:
      count = 0
      items.sort()
      for count in items:
          pass
      return count
  ```

- **A name read by a nested function, lambda or class**: the nested scope may read it whenever it is called, so the name
  must exist from the assignment onwards. One that binds the name for itself, as a parameter or an assignment, reads its
  own variable and does not count.

  ```python ok
  def collect(items: list[int]) -> list[int]:
      found = []
      items.sort()

      def add() -> None:
          found.append(1)

      add()
      return found
  ```

- **A use outside the block, or a use before the assignment**: a loop may read the name on its next pass, and an
  exception handler outside the block may read it after the block is left by an exception, so the constant would not be
  set on every path.

  ```python ok
  def fill(items: list[int]) -> list[int]:
      out = []
      while items:
          print(out)
          out = [1]
          items.pop()
      return out
  ```

  ```python ok
  def fill(items: list[int]) -> list[int]:
      try:
          out = []
          items.sort()
          out.append(1)
      except ValueError:
          return out
      return out
  ```

- **A function that reads its locals by name**, such as with
  [`locals()`](https://docs.python.org/3/builtins/functions.html#locals),
  [`vars()`](https://docs.python.org/3/builtins/functions.html#vars),
  [`eval()`](https://docs.python.org/3/builtins/functions.html#eval) or
  [`exec()`](https://docs.python.org/3/builtins/functions.html#exec): moving the assignment changes what they see.

  ```python ok
  def run(items: list[int]) -> None:
      seed = 2
      items.sort()
      print(locals())
  ```

- **A string between the assignment and the use that spells the name**: code may run it as source, as `exec` does. A
  string that keys a subscript or a dict display is data and does not count.

  ```python ok
  def run(items: list[int]) -> list[int]:
      out = []
      print("out.append(1)")
      out.extend(items)
      return out
  ```

- **A comment between the assignment and the use**: moving the statement would move the comment's meaning with it.

  ```python ok
  def total(items: list[int]) -> int:
      count = 0  # the running total
      items.sort()
      return count + len(items)
  ```

- **A comment on the line above the assignment**: the comment explains the assignment, and it would stay behind when the
  assignment moves. The assignment is still reported, but no fix is offered.

  ```python report
  def run(items, prepare, log, send):
      # Retries allowed before giving up: the upstream API drops one call in three.
      limit = 3
      prepare(items)
      log(items)
      return send(items, limit)
  ```

  ```python fixed
  def run(items, prepare, log, send):
      # Retries allowed before giving up: the upstream API drops one call in three.
      limit = 3
      prepare(items)
      log(items)
      return send(items, limit)
  ```

- **Another constant assignment before the first use.** Moving constants past one another can make repeated fixes undo
  each other. The rule skips the earlier assignment.

  ```python ok
  def total(items: list[int]) -> int:
      count = 0
      limit = 3
      return count + limit + len(items)
  ```

- **Assignments at module level**: another module can read a module attribute while an import runs the statements in
  between.

  ```python ok
  LIMIT = 3
  print("starting")
  print(LIMIT)
  ```

- **A name that is `_` or a dunder**: `_` is a placeholder, and a dunder has a meaning to Python.

  ```python ok
  def run(items, prepare, send):
      _ = 3
      prepare(items)
      return send(items, _)
  ```

  ```python ok
  def run(items, prepare, send):
      __retries__ = 3
      prepare(items)
      return send(items, __retries__)
  ```

### Fix

The safe fix moves the assignment immediately before the statement that reads the name. The assignment must occupy its
own lines. If the destination shares a line with other statements separated by `;`, the fix splits the line. A trailing
comment on the destination can prevent the fix.

## Related rules

- [single-use-variable](SMP504.html) inlines a value read by the statement right below its assignment.
- [collection-built-by-statements](SMP531.html) writes an empty collection's entries in its literal.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** issue [#11060](https://github.com/astral-sh/ruff/issues/11060) tracks
  Sourcery refactorings.
- [Sourcery](https://sourcery.ai) moves an assignment down to its first use through
  [move-assign](https://docs.sourcery.ai/reference/refactorings/python/move-assign/). simplint only moves constants.
