Flags zero-argument `super()` calls placed where Python cannot find the class or the instance.

## Why is this bad?

Zero-argument `super()` uses the enclosing class and the first positional argument of the function where it runs. In a
method that argument is normally `self` or `cls`. In a generator expression it is the iterator. A parameterless nested
function has no first argument, and a module-level function has no enclosing class. A call with no suitable binding can
raise [`TypeError` or `RuntimeError`](https://docs.python.org/3/builtins/functions.html#super).

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

- **List, set and dict comprehensions** run in their own function before Python 3.12. Python 3.12 inlines them in the
  surrounding scope. [PEP 709](https://peps.python.org/pep-0709/)

- **Lambdas and nested functions without positional parameters or `*args`**.

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

- **The first iterable of a comprehension** uses the enclosing scope. Inside a method, `super()` there can use the
  method's first argument.

  ```python ok
  class Child(dict):
      def doubled(self) -> list[str]:
          return [key * 2 for key in super().keys()]
  ```

- **Explicit arguments to `super()`** put the call outside this rule.

  ```python ok
  class Child(dict):
      def sizes(self) -> list[int]:
          return list(super(Child, self).__len__() for _ in range(3))
  ```

## Configuration

- The [Python target version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)
  determines whether list, set and dict comprehensions count.

## Other tools

- Ruff's [UP008](https://docs.astral.sh/ruff/rules/super-call-with-parameters/) checks for redundant arguments in calls
  to `super`.
