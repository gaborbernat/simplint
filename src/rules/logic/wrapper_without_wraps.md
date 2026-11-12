Flags a decorator whose wrapper forwards its `*args` or `**kwargs` to the decorated function without
[`functools.wraps`](https://docs.python.org/3/library/functools.html#functools.wraps).

## Why is this bad?

A decorator returns the wrapper in place of the function, so the decorated name now holds a function called `wrapper`
with no docstring and a `(*args, **kwargs)` signature. Tracebacks, logs, `help()`, Sphinx autodoc and anything that
reads `inspect.signature` see the wrapper instead of the function. `functools.wraps` copies the name, docstring and
module across and sets `__wrapped__`, through which `inspect.signature` finds the original signature.

## Example

```python report
def logged(func):
    def wrapper(*args, **kwargs):
        print(func.__name__)
        return func(*args, **kwargs)

    return wrapper
```

Use instead:

```python ok
import functools


def logged(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        print(func.__name__)
        return func(*args, **kwargs)

    return wrapper
```

## What it does

### Active on

- **A function defined in another function**, with `*args` or `**kwargs`, that calls one of the outer function's
  parameters passing its own `*args` or `**kwargs` and nothing but its own parameters, and that the outer function
  returns. Decorator factories count too: the check runs on each level.

  **Unsafe fix:** adds `@functools.wraps(func)` above the wrapper, or `@wraps(func)` when the file already imports
  `wraps` from `functools`, and imports `functools` when the file lacks it. The wrapper then reports the wrapped
  function's name and signature, which changes what code that reads them sees.

  ```python report
  def logged(func):
      def wrapper(*args, **kwargs):
          return func(*args, **kwargs)

      return wrapper
  ```

  ```python fixed
  import functools


  def logged(func):
      @functools.wraps(func)
      def wrapper(*args, **kwargs):
          return func(*args, **kwargs)

      return wrapper
  ```

### Exceptions

- **A wrapper that already copies the metadata**, through a decorator of its own, `functools.update_wrapper` or an
  assignment to one of its dunders, such as `wrapper.__name__ = func.__name__`.

- **An adapter**, which calls the function with arguments of another shape, as `func(len(args), *args)` or
  `func(**options)` from a local `options`. `functools.wraps` would then report a signature the wrapper does not have.

  ```python ok
  def counted(func):
      def wrapper(*args):
          return func(len(args), *args)

      return wrapper
  ```

- **A closure with a fixed signature**, as a callback factory makes, which keeps a name of its own.

## Ruff overlap

Ruff has no rule for a wrapper that lacks `functools.wraps`.

## Other tools

No Ruff, Pylint or flake8 rule reports it.
