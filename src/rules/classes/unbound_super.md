Flags zero-argument `super()` calls placed where Python cannot find the class or the instance.

## Why is this bad?

`super()` without arguments reads the `__class__` cell the compiler creates for methods, and the first argument of the
function it runs in. In a generator expression that argument is the iterator the expression loops over, a lambda or
nested function without parameters has none, and a function outside a class has no `__class__` cell. Each call raises
`TypeError` or `RuntimeError` only when that line runs.

## Example

```python report
class Child(dict):
    def sizes(self) -> list[int]:
        return list(super().__len__() for _ in range(3))
```

Use instead:

```python ok
class Child(dict):
    def sizes(self) -> list[int]:
        size = super().__len__()
        return [size for _ in range(3)]
```

## What it does

### Active on

- **Generator expressions**, at any target version.
- **List, set and dict comprehensions** before Python 3.12, which run them as functions of their own; 3.12 inlined
  them ([PEP 709](https://peps.python.org/pep-0709/)).
- **Lambdas and nested functions without positional parameters**.

  ```python report
  class Child(dict):
      def size(self) -> int:
          return (lambda: super().__len__())()
  ```

- **Class bodies and functions outside any class**.

  ```python report
  def size() -> int:
      return super().__len__()
  ```

### Exceptions

- **The first iterable of a comprehension** runs in the enclosing function, so `super()` there works.

  ```python ok
  class Child(dict):
      def doubled(self) -> list[str]:
          return [key * 2 for key in super().keys()]
  ```

- **`super()` with arguments** names the class and instance itself.

  ```python ok
  class Child(dict):
      def sizes(self) -> list[int]:
          return list(super(Child, self).__len__() for _ in range(3))
  ```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) decides whether comprehensions count; see
  [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Its
  [super-call-with-parameters](https://docs.astral.sh/ruff/rules/super-call-with-parameters/) asks for the zero-argument
  form, and leaves these places alone.
