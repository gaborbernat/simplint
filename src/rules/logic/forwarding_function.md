Flags private functions and methods whose body only forwards their parameters to another call.

## Why is this bad?

The wrapper adds a name and an extra hop without adding behavior, and a reader has to open it to learn that it calls
something else. [John Ousterhout](https://web.stanford.edu/~ouster/cgi-bin/book.php) lists such pass-through methods as
a red flag: they add an interface without adding function, and a change to the target's parameters must be repeated in
the wrapper. Calling the target itself removes the indirection. Ruff's
[unnecessary-lambda](https://docs.astral.sh/ruff/rules/unnecessary-lambda/) asks the same when the forward is a lambda.
[Martin Fowler](https://martinfowler.com/bliki/FunctionLength.html) defends one-line functions whose name states their
intent. The check cannot judge whether a name adds meaning, so it flags private forwards.

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

No fix is offered, since inlining needs each call site's arguments mapped onto the target's parameters.

### Active on

A function is reported when its body, apart from a docstring, is a single call that is returned or used as an expression
statement, and every argument of the call is one of the function's own parameters, passed as is, as `*args` or
`**kwargs` ([arbitrary argument lists](https://docs.python.org/3/tutorial/controlflow.html#arbitrary-argument-lists)),
or as `keyword=parameter`. The call target must be a name or a dotted name such as `self.client.get`.

- **Module-level functions whose name starts with `_`**: their callers are all in the file, so each can call the target
  directly.

  ```python report
  def load(name: str) -> Path:
      return _make_path(name)


  def _make_path(name: str) -> Path:
      return Path(name)
  ```

- **Methods with such names in module-level classes without base classes**: matched through `self.name`, `cls.name` or
  `ClassName.name`.

  ```python report
  class Store:
      def load(self, name: str) -> Path:
          return self._make_path(name)

      def _make_path(self, name: str) -> Path:
          return Path(name)
  ```

- **A call whose `(` starts a later line**: the callee is the name the call reads, whatever line the `(` is on, so the
  call is reported like one on a single line.

  ```python report
  def load(name: str) -> Path:
      return _make_path(name)


  def _make_path(name: str) -> Path:
      return Path(name)
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

- **A reference that ends its line**: a line that starts with `(` begins a new statement, so the name is only read there
  and not called.

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

  - a parameter default: a default on a parameter of the wrapped function, which the wrapper does not pass.

    ```python ok
    def load(name: str) -> IO[str]:
        return _open(name)


    def _open(name: str, mode: str = "r") -> IO[str]:
        return open(name, mode)
    ```

- **Functions another file in the run patches by name**, such as with `mocker.patch("pkg.module._load")`: tests replace
  them, so they must keep existing.

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

- **Methods whose target goes through the receiver**, such as `self.client.get`: the receiver is the object the method
  is called on, so the target called there is the same one.

  ```python report
  class Cache:
      def __init__(self, client):
          self.client = client

      def fetch(self, name: str) -> bytes:
          return self._get(name)

      def _get(self, name: str) -> bytes:
          return self.client.get(name)
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

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Released rule
  [PLW0108](https://docs.astral.sh/ruff/rules/unnecessary-lambda/) reports the same forward written as a lambda.
