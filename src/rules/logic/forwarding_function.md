Flags private functions and methods whose body only forwards their parameters to another call.

## Why is this bad?

A reader who follows a call to `_make_path(name)` finds another call, `Path(name)`. Calling `Path` directly shows which
constructor runs and removes a signature to maintain.
[John Ousterhout](https://web.stanford.edu/~ouster/cgi-bin/cs190-spring16/lecture.php?topic=editorReview) describes this
cost for pass-through methods. [Martin Fowler](https://martinfowler.com/bliki/FunctionLength.html) defends one-line
functions whose name states their intent. The check cannot judge whether a name adds meaning, so it flags private
forwards.

## Example

```python report
def config_path() -> Path:
    return _make_path("config.toml").resolve()


def _make_path(name: str) -> Path:
    return Path(name)
```

Use instead:

```python ok
def config_path() -> Path:
    return Path("config.toml").resolve()
```

## What it does

The rule offers no automatic fix. At each call site, check how the wrapper maps its arguments and whether it returns or
discards the result.

### Active on

A function is reported when its body, apart from a docstring, is a single call that is returned or used as an expression
statement, and every argument of the call is one of the function's own parameters, passed as is, as `*args` or
`**kwargs` ([arbitrary argument lists](https://docs.python.org/3/tutorial/controlflow.html#arbitrary-argument-lists)),
or as `keyword=parameter`. The call target must be a name or a dotted name such as `self.client.get`. Calls split across
lines count too.

- **Private module-level functions.** The rule looks for calls in the defining file.

  ```python report
  def load(name: str) -> Path:
      return _make_path(name)


  def _make_path(name: str) -> Path:
      return Path(name)
  ```

- **Private methods in module-level classes without explicit base classes.** The rule recognizes references through
  `self.name`, `cls.name` or `ClassName.name`.

  ```python report
  class Store:
      def load(self, name: str) -> Path:
          return self._make_path(name)

      def _make_path(self, name: str) -> Path:
          return Path(name)
  ```

- **Methods that call an attribute of their receiver** can also be reported. In this example, `fetch` can call
  `self.client.get` directly.

  ```python report
  class Cache:
      def __init__(self, client):
          self.client = client

      def fetch(self, name: str) -> bytes:
          return self._get(name)

      def _get(self, name: str) -> bytes:
          return self.client.get(name)
  ```

### Exceptions

- **[Dunder](https://docs.python.org/3/reference/datamodel.html#special-method-names) functions**: Python calls them by
  name, so there is no call site to rewrite.

  ```python ok
  def __repr__(value: object) -> str:
      return str(value)
  ```

- **Decorated functions**: the decorator wraps or registers the function, so it does more than forward.

  ```python ok
  import functools


  def load(name: str) -> Path:
      return _make_path(name)


  @functools.cache
  def _make_path(name: str) -> Path:
      return Path(name)
  ```

- **Functions the file never refers to, or refers to anywhere without calling**, for example when passing one as a
  callback: such a reference needs the function itself, which a direct call to the target cannot replace.

  ```python ok
  def load(names: list[str]) -> list[Path]:
      return list(map(_make_path, names))


  def _make_path(name: str) -> Path:
      return Path(name)
  ```

- **Methods of a class another class derives from**: a subclass may override the method, and a call through the base
  class would then reach the override rather than the target.

  ```python ok
  class Store:
      def load(self, name: str) -> Path:
          return self._make_path(name)

      def _make_path(self, name: str) -> Path:
          return Path(name)


  class RemoteStore(Store):
      def _make_path(self, name: str) -> Path:
          return Path("remote") / name
  ```

- **A reference that ends its statement.** A `(` on a new statement cannot turn the preceding reference into a call.

  ```python ok
  def load(name: str) -> Path:
      callback = _make_path
      (callback)(name)


  def _make_path(name: str) -> Path:
      return Path(name)
  ```

- **The receiver itself**: a method that calls its receiver, such as `self(name)`, forwards to the object the method
  belongs to, and no call site can name that object in place of the method.

  ```python ok
  class Cache:
      def fetch(self, name: str) -> object:
          return self._own(name)

      def _own(self, name: str) -> object:
          return self(name)
  ```

- **Wrappers that add their own values**: they configure the call, so they are not a pure forward. These additions
  count:

  - `check=True`: a keyword argument the wrapper sets itself.

    ```python ok
    def load(text: str) -> str:
        return _run(text)


    def _run(text: str) -> str:
        return subprocess.run(text, check=True)
    ```

  - a computed argument: a value the wrapper works out before the call.

  - a default on any parameter of the wrapper itself, such as `mode="r"` below.

    ```python ok
    def load(name: str) -> IO[str]:
        return _open(name)


    def _open(name: str, mode: str = "r") -> IO[str]:
        return open(name, mode)
    ```

- **Functions referenced by another file in the run.** This includes imports and patches such as
  `mocker.patch("pkg.module._load")`. Removing the function would break that reference.

  ```python ok file=service.py
  def load(name: str) -> Path:
      return _make_path(name)


  def _make_path(name: str) -> Path:
      return Path(name)
  ```

  ```python context file=test_service.py
  from unittest import mock

  from service import load


  def test_load() -> None:
      with mock.patch("service._make_path"):
          assert load("a")
  ```

- **Calls on a computed value**, such as `make(name).strip()`: this is a call on the result of another call, not a
  forward to a named function.

  ```python ok
  def load(name: str) -> str:
      return _clean(name)


  def _clean(name: str) -> str:
      return make(name).strip()
  ```

- **Calls whose target starts with one of the function's own parameters**, such as `fn` in
  `def _apply(fn, x): return fn(x)`: the call runs whatever the caller passed in, so there is no fixed target to call
  instead.

  ```python ok
  def load(text: str) -> str:
      return _apply(str.strip, text)


  def _apply(fn: Callable[[str], str], text: str) -> str:
      return fn(text)
  ```

## Other tools

- Ruff's [PLW0108](https://docs.astral.sh/ruff/rules/unnecessary-lambda/) reports lambdas that forward their arguments.
