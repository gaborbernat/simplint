Flags a decorator whose wrapper forwards its `*args` or `**kwargs` to the decorated function without
[`functools.wraps`](https://docs.python.org/3/library/functools.html#functools.wraps).

## Why is this bad?

Without `functools.wraps`, [`help()`](https://docs.python.org/3/builtins/functions.html#help) displays the wrapper's
name and docstring, and [`inspect.signature()`](https://docs.python.org/3/library/inspect.html#inspect.signature) sees
the wrapper's parameters. `wraps` copies metadata from the decorated function and sets `__wrapped__`, which lets
introspection follow the wrapper to the original signature.

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

- **A nested function that the outer function returns.** The nested function calls one of the outer function's
  parameters, forwarding its own `*args` or `**kwargs`. All arguments to that call must come from the wrapper's
  parameters. Simplint checks each level of a decorator factory.

  **Unsafe fix:** adds `@functools.wraps(func)` above the wrapper, or `@wraps(func)` when the file already imports
  `wraps` from `functools`, and imports `functools` when the file lacks it. Tools inspecting the wrapper then see the
  original function's metadata.

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

- **A decorated wrapper.** Simplint also skips the wrapper if the enclosing decorator calls a function named
  `update_wrapper` or assigns to an attribute of the wrapper starting with `__`, such as `wrapper.__name__`. These are
  signs that the decorator may already manage the metadata.

- **An adapter**, which calls the function with arguments of another shape, as `func(len(args), *args)` or
  `func(**options)` from a local `options`. `functools.wraps` would then report a signature the wrapper does not have.

  ```python ok
  def counted(func):
      def wrapper(*args):
          return func(len(args), *args)

      return wrapper
  ```

- **A closure with a fixed signature**, as a callback factory makes, which keeps a name of its own.

## Other tools

The [Ruff rule list](https://docs.astral.sh/ruff/rules/) has no equivalent check for a decorator that omits
`functools.wraps`.
