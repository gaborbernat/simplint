Flags exception classes whose `__init__` cannot be called again with the `args` it hands to its parent.

## Why is this bad?

By default, [`pickle`](https://docs.python.org/3/library/pickle.html) and
[`copy`](https://docs.python.org/3/library/copy.html) reconstruct an exception by calling its class with the positional
arguments in `exception.args`. In the example, `ParseError` requires a path and a line number but stores one formatted
string in `args`. Unpickling calls `ParseError(message)` and raises
[`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError) because the line number is missing. This can
prevent a process pool from returning the original exception to its caller.

## Example

```python report
class ParseError(ValueError):
    def __init__(self, path: str, line: int) -> None:
        super().__init__(f"{path}:{line}")
        self.path = path
        self.line = line
```

Use instead:

```python ok
class ParseError(ValueError):
    def __init__(self, path: str, line: int) -> None:
        super().__init__(path, line)
        self.path = path
        self.line = line

    def __str__(self) -> str:
        return f"{self.path}:{self.line}"
```

## What it does

### Active on

- **A parent `__init__` call** through `super().__init__(...)`, `super(Class, self).__init__(...)` or
  `Base.__init__(self, ...)` whose positional arguments fall outside the number the subclass's own `__init__` accepts.

  ```python report
  class Closed(Exception):
      def __init__(self) -> None:
          super().__init__("the stream is closed")
  ```

- **A required keyword-only parameter**, which `args` cannot carry.

  ```python report
  class RequestError(Exception):
      def __init__(self, message: str, *, request: object) -> None:
          super().__init__(message)
          self.request = request
  ```

The class must derive from [`BaseException`](https://docs.python.org/3/builtins/exceptions.html#BaseException) through
builtin, standard library or same-file classes, or be one the project index knows as an exception.

### Exceptions

- **Custom reconstruction hooks**. The rule skips a class that defines `__reduce__`, `__reduce_ex__`, `__getnewargs__`
  or `__getnewargs_ex__`, or inherits one from a base in the same file. To control how an exception is rebuilt, define
  `__reduce__` as in this example.

  ```python ok
  class Closed(Exception):
      def __init__(self) -> None:
          super().__init__("the stream is closed")

      def __reduce__(self) -> tuple[type, tuple[()]]:
          return type(self), ()
  ```

- **Calls that unpack** `*args` or `**kwargs`, whose count simplint cannot know.

  ```python ok
  class Wrapped(Exception):
      def __init__(self, *args: object) -> None:
          super().__init__(*args)
  ```

## Other tools

- Ruff's [rule list](https://docs.astral.sh/ruff/rules/) has no equivalent check for exception constructor arguments.
