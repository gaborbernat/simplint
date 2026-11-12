Flags a function used as a condition without calling it, as `if is_ready:` where `if is_ready():` was meant.

## Why is this bad?

A function object is always true. The condition never depends on what the function would return, so the branch always
runs, a `while not ready` loop never runs, and an `assert` always passes.

## Example

```python report
def is_ready() -> bool:
    return False


def start() -> str:
    if is_ready:
        return "started"
    return "waiting"
```

Use instead:

```python ok
def is_ready() -> bool:
    return False


def start() -> str:
    if is_ready():
        return "started"
    return "waiting"
```

## What it does

### Active on

- **Tests of `if`, `elif`, `while`, `assert` and conditional expressions**, alone, negated with `not`, or as an operand
  of `and` or `or`. The name must be a function of the same file, or of another module of the run the file imports, or
  `self.method` inside a method of the class that defines it.

  **Unsafe fix:** when the function takes no arguments, the fix calls it, which runs its body where the test is. A
  function that needs arguments, or one from another module, is reported without a fix.

  ```python report
  class Queue:
      def is_empty(self) -> bool:
          return True

      def drain(self) -> None:
          while not self.is_empty:
              pass
  ```

  ```python fixed
  class Queue:
      def is_empty(self) -> bool:
          return True

      def drain(self) -> None:
          while not self.is_empty():
              pass
  ```

### Exceptions

- **Decorated functions**, since a decorator such as `property` turns the name into the value it computes.

  ```python ok
  class Queue:
      @property
      def is_empty(self) -> bool:
          return True

      def drain(self) -> None:
          while not self.is_empty:
              pass
  ```

- **Attributes of other objects** and names the run cannot resolve to a `def`, such as `os.path.exists`.

## Ruff overlap

Ruff has no rule for a function used as a condition.

## Other tools

- **[Pylint](https://pylint.readthedocs.io/en/stable/user_guide/messages/warning/using-constant-test.html):**
  `using-constant-test` (W0125) reports a function, among other constants, as the test of an `if` or a conditional
  expression.
