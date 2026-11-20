Flags adjacent property and method decorators that can give the getter the wrong arguments or change what attribute
access returns.

## Why is this bad?

A property calls its getter with the instance. A `classmethod` descriptor used as that getter is not callable. On Python
3.10 and later a `staticmethod` descriptor is callable, but a getter defined with no parameters still raises
[`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError) when it receives the instance. Reversing
`@property` and `@classmethod` is version-dependent. Python deprecated class methods wrapping other descriptors in 3.11
and removed support in 3.13. On 3.13 and later `Model.kind` in the example returns a bound method.
[Python's `classmethod` and `staticmethod` documentation](https://docs.python.org/3/builtins/functions.html#classmethod)

## Example

```python report
class Model:
    @classmethod
    @property
    def kind(cls) -> str:
        return "model"
```

Use instead:

```python ok
class Model:
    kind = "model"
```

## What it does

### Active on

- **`@property` or `@functools.cached_property` over `@staticmethod` or `@classmethod`**, at any target version.

  ```python report
  class Model:
      @property
      @staticmethod
      def size() -> int:
          return 1
  ```

- **`@classmethod` over `@property` or `@functools.cached_property`** when the target version is 3.11 or later.

### Exceptions

- **Functions outside a class body** can supply descriptors that code later assigns to a class.

  ```python ok
  @classmethod
  def from_pairs(cls, pairs: list[tuple[str, int]]) -> object:
      return cls(pairs)
  ```

## Configuration

- The [Python target version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)
  determines when simplint reports `@classmethod` above a property.

  ```toml config
  [tool.ruff]
  target-version = "py310"
  ```

  ```python ok
  class Model:
      @classmethod
      @property
      def kind(cls) -> str:
          return "model"
  ```

## Other tools

- Ruff's preview rule [RUF074](https://docs.astral.sh/ruff/rules/incorrect-decorator-order/) checks the placement of
  `@abstractmethod`.
