Flags modules where the last top-level statement that binds `__all__` is not the last statement.

## Why is this bad?

Readers look for the export list in one place. At the end of the module it comes after the definitions it names, so it
reads as a summary of what the module provides. [PEP 8](https://peps.python.org/pep-0008/#module-level-dunder-names)
asks for `__all__` after the docstring and before the imports, the placement this rule reports; simplint keeps the
report because a list at the top cannot summarize the definitions below it. [missing-from-all](SMP401.html) and
[all-format](SMP405.html) also check `__all__`.

## Example

```python report
__all__ = ["main"]


def main() -> None:
    run()


if __name__ == "__main__":
    main()
```

Use instead:

```python ok
def main() -> None:
    run()


__all__ = ["main"]


if __name__ == "__main__":
    main()
```

## What it does

### Active on

- **Bindings of [`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package) followed by other
  statements**: the last top-level statement that binds `__all__` is reported when a statement comes after it, other
  than a call on `__all__` (see Exceptions). An assignment, an annotated assignment or an augmented assignment binds it.

  - an assignment, followed by a call:

    ```python report
    __all__ = ["main"]


    def main() -> None:
        pass


    main()
    ```

  - an annotated assignment, followed by a call:

    ```python report
    __all__: list[str] = ["main"]


    def main() -> None:
        pass


    main()
    ```

  - an augmented assignment such as `+=`, followed by a `def`:

    ```python report
    __all__ = []
    __all__ += ["main"]


    def main() -> None:
        pass
    ```

  - a trailing `if` other than a `__main__` block, such as `if __name__ != "__main__":`, which counts as a regular
    statement:

    ```python report
    __all__ = ["main"]


    def main() -> None:
        pass


    if __name__ != "__main__":
        main()
    ```

### Exceptions

- **A trailing [`if __name__ == "__main__":`](https://docs.python.org/3/library/__main__.html#idiomatic-usage) block**:
  it may follow `__all__`, since it runs the module as a script once everything above it is defined.

  - written as `if __name__ == "__main__":`:

    ```python ok
    def main() -> None:
        pass


    __all__ = ["main"]


    if __name__ == "__main__":
        main()
    ```

  - with the operands swapped, as `if "__main__" == __name__:`:

    ```python ok
    def main() -> None:
        pass


    __all__ = ["main"]


    if "__main__" == __name__:
        main()
    ```

- **Calls on `__all__` after its binding**, such as `__all__.extend(...)` or `__all__.append(...)`: a call changes the
  list the binding made, so the calls stay with it. The binding and its calls form one block, which must still be last.

  ```python ok
  def main() -> None:
      pass


  __all__ = ["main"]
  __all__.extend(["stop"])
  __all__.append("halt")
  ```

  ```python report
  __all__ = ["main"]
  __all__.append("stop")


  def main() -> None:
      pass
  ```

### Fix

The fix is safe. It moves the statement to the end of the module, before a trailing `__main__` block, with two blank
lines before it, also when the file has no trailing newline. Comments further up stay where they are. A comment on the
line directly above the statement blocks the fix, and so does one above a `# fmt: off` / `# fmt: on` pair around the
statement; the pair itself moves with it.

```python report
__all__ = ["main"]


def main() -> None:
    run()


if __name__ == "__main__":
    main()
```

```python fixed
def main() -> None:
    run()


__all__ = ["main"]


if __name__ == "__main__":
    main()
```

The fix is offered only when:

- a single statement binds `__all__`, to a list or tuple of string literals
- no later statement reads `__all__` or calls a method on it
- no later statement reads `__all__` at run time, through an attribute such as `sys.modules[__name__].__all__`, a
  `"__all__"` string or a call to [`globals()`](https://docs.python.org/3/builtins/functions.html#globals),
  [`vars()`](https://docs.python.org/3/builtins/functions.html#vars) or
  [`dir()`](https://docs.python.org/3/builtins/functions.html#dir)
- no function or lambda reads `__all__`, by name or at run time, wherever it is defined
- the statement shares no line with other code
- no comment sits on the line above it, or above the `# fmt: off` / `# fmt: on` pair around it

With several bindings or a later reader, moving the statement could change the value of `__all__` or what that reader
sees:

```python report
__all__ = ["main"]
__all__ += ["stop"]


def main() -> None:
    pass
```

```python fixed
__all__ = ["main"]
__all__ += ["stop"]


def main() -> None:
    pass
```

```python report
__all__ = ["main"]


def main() -> None:
    pass


print(__all__)
```

```python fixed
__all__ = ["main"]


def main() -> None:
    pass


print(__all__)
```

A module that reads `__all__` at run time still needs it bound above, so the statement is reported without a fix when
the module reads it through:

- an attribute, such as `sys.modules[__name__].__all__`:

  ```python report
  import sys

  __all__ = ["main"]


  def main() -> None:
      pass


  print(sys.modules[__name__].__all__)
  ```

  ```python fixed
  import sys

  __all__ = ["main"]


  def main() -> None:
      pass


  print(sys.modules[__name__].__all__)
  ```

- the string `"__all__"`, such as in
  [`getattr(sys.modules[__name__], "__all__")`](https://docs.python.org/3/builtins/functions.html#getattr):

  ```python report
  import sys

  __all__ = ["main"]


  def main() -> None:
      pass


  print(getattr(sys.modules[__name__], "__all__"))
  ```

  ```python fixed
  import sys

  __all__ = ["main"]


  def main() -> None:
      pass


  print(getattr(sys.modules[__name__], "__all__"))
  ```

- a call to `globals()`:

  ```python report
  __all__ = ["main"]


  def main() -> None:
      pass


  print(sorted(globals()))
  ```

  ```python fixed
  __all__ = ["main"]


  def main() -> None:
      pass


  print(sorted(globals()))
  ```

- a call to `vars()`:

  ```python report
  __all__ = ["main"]


  def main() -> None:
      pass


  print(sorted(vars()))
  ```

  ```python fixed
  __all__ = ["main"]


  def main() -> None:
      pass


  print(sorted(vars()))
  ```

- a call to `dir()`:

  ```python report
  __all__ = ["main"]


  def main() -> None:
      pass


  print(dir())
  ```

  ```python fixed
  __all__ = ["main"]


  def main() -> None:
      pass


  print(dir())
  ```

- a function or lambda that reads it when called, by name or with `globals()`, `vars()` or `dir()`, even one defined
  above the statement:

  ```python report
  def check() -> list[str]:
      return [globals()[name] for name in __all__]


  __all__ = ["check"]


  RESULT = check()
  ```

  ```python fixed
  def check() -> list[str]:
      return [globals()[name] for name in __all__]


  __all__ = ["check"]


  RESULT = check()
  ```

- a lambda that calls `dir()`, even one defined above the statement:

  ```python report
  read = lambda: sorted(dir())


  __all__ = ["check"]


  def check() -> list[str]:
      return read()
  ```

  ```python fixed
  read = lambda: sorted(dir())


  __all__ = ["check"]


  def check() -> list[str]:
      return read()
  ```

A value that depends on where it runs, such as a comprehension over `dir()`, gives another list once the statement
moves, so only a list or tuple of string literals is moved:

```python report
__all__ = [name for name in dir() if not name.startswith("_")]


def helper() -> str:
    return "helper"
```

```python fixed
__all__ = [name for name in dir() if not name.startswith("_")]


def helper() -> str:
    return "helper"
```

A comment on the line above the statement would stay behind on the line above the next statement, so the statement is
reported without a fix:

```python report
import os


# The public API of this module.
__all__ = ["helper"]


def helper() -> str:
    return os.sep
```

```python fixed
import os


# The public API of this module.
__all__ = ["helper"]


def helper() -> str:
    return os.sep
```

The same holds for a comment above a `# fmt: off` / `# fmt: on` pair around the statement, which would stay behind as
well:

```python report
import os


# The public API of this module.
# fmt: off
__all__ = ["helper"]
# fmt: on


def helper() -> str:
    return os.sep
```

```python fixed
import os


# The public API of this module.
# fmt: off
__all__ = ["helper"]
# fmt: on


def helper() -> str:
    return os.sep
```

A statement that shares its line with other code is reported without a fix as well:

```python report
# fmt: off
__all__ = ["main"]; VERSION = "1"
# fmt: on


def main() -> None:
    pass
```

```python fixed
# fmt: off
__all__ = ["main"]; VERSION = "1"
# fmt: on


def main() -> None:
    pass
```

### Related rules

[all-format](SMP405.html) checks the shape of `__all__` itself: a list, not a tuple, with a trailing comma.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
