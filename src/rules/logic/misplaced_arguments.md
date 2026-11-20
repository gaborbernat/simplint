Flags a call that passes two or more variables by position into the slots of other parameters of the same function,
where the variables are named after those parameters.

## Why is this bad?

Passing a positional argument twice can shift the remaining values into the wrong parameters. In the example, `shutdown`
receives a boolean as its cancellation message and a string as its exception type.

## Example

```python report
def shutdown(cancel: bool, cancel_msg: str, exc_type: type = RuntimeError) -> None: ...


def stop(cancel: bool, cancel_msg: str) -> None:
    shutdown(cancel, cancel, cancel_msg)
```

Use instead:

```python ok
def shutdown(cancel: bool, cancel_msg: str, exc_type: type = RuntimeError) -> None: ...


def stop(cancel: bool, cancel_msg: str) -> None:
    shutdown(cancel, cancel_msg)
```

## What it does

### Active on

- **Calls by name** of a function of the same file, or of one another module of the run defines and the file imports,
  and **calls of `self.method()`** inside a method of the class that defines it. Only positional arguments that are bare
  names count, and only parameters the call fills by position.

### Exceptions

- **Arguments that only trade places among themselves**, as `move(target, source)` for `move(source, target)` does. A
  test of a symmetric function or a call that flips two roles swaps on purpose, so such a call is not reported.

  ```python ok
  def distance(start: float, end: float) -> float:
      return abs(end - start)


  def symmetric(start: float, end: float) -> bool:
      return distance(start, end) == distance(end, start)
  ```

- **One misplaced argument.** A call such as `notify(user, user)` may intentionally pass the same value twice.

- **Decorated functions** and calls with `*args`, whose parameters the definition no longer tells.

## Ruff overlap

Ruff has no rule for misplaced arguments.

## Other tools

- **[Pylint](https://pylint.readthedocs.io/en/stable/user_guide/messages/warning/arguments-out-of-order.html):**
  `arguments-out-of-order` (W1114) checks calls whose positional argument names match the parameter names in a different
  order. This includes swaps such as `distance(end, start)`.
