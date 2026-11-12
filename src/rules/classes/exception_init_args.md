Flags exception classes whose `__init__` cannot be called again with the `args` it hands to its parent.

## Why is this bad?

[`pickle`](https://docs.python.org/3/library/pickle.html) and [`copy`](https://docs.python.org/3/library/copy.html)
rebuild an exception by calling its class with its `args`, which `BaseException.__init__` sets to the arguments it
receives. A subclass that takes a path but passes its parent a formatted message, or that requires a keyword-only
argument, raises `TypeError` when it is unpickled. That breaks exceptions sent across
[`multiprocessing`](https://docs.python.org/3/library/multiprocessing.html) and
[`concurrent.futures`](https://docs.python.org/3/library/concurrent.futures.html) workers, where the original error is
replaced by a confusing one about the rebuild.

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

- **A parent `__init__` call** through `super().__init__(...)` or `Base.__init__(self, ...)` whose positional
  arguments fall outside the number the subclass's own `__init__` accepts.

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

The class must derive from `BaseException` through builtin, standard library or same-file classes, or be one the project
index knows as an exception.

### Exceptions

- **A rebuilding method**: a class that defines, or inherits from a same-file base, `__reduce__`, `__reduce_ex__`,
  `__getnewargs__` or `__getnewargs_ex__` tells `pickle` and `copy` how to rebuild it.

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

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
