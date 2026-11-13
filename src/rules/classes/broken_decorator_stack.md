Flags descriptor decorators stacked so that the outer one cannot wrap what the inner one returns.

## Why is this bad?

Decorators apply bottom up, so the outer one wraps the descriptor the inner one returns. `@property` over
`@staticmethod` or `@classmethod` calls the wrapped object with the instance, which it does not accept, so reading the
attribute raises `TypeError`. `@classmethod` over `@property` chained descriptors on Python 3.9 and 3.10; Python 3.11
deprecated it and Python 3.13 removed it, so the attribute now reads as a bound method instead of the value.

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

- **Functions outside a class body**: a descriptor built at module level is often set on a class later, as
  `cls.__subclasshook__ = _proto_hook` does.

  ```python ok
  @classmethod
  def from_pairs(cls, pairs: list[tuple[str, int]]) -> object:
      return cls(pairs)
  ```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version); see
  [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

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

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Its preview rule
  [RUF074](https://docs.astral.sh/ruff/rules/incorrect-decorator-order/) covers the order around `@abstractmethod`,
  which this rule leaves to it.
