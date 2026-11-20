Flags mocks and patches that lack a spec or use one that does not check call signatures.

## Why is this bad?

If production code renames a method or adds a required argument, a plain mock can still accept the old call.
[Autospeccing](https://docs.python.org/3/library/unittest.mock.html#autospeccing) restricts attribute access and checks
the signatures it can inspect.

## Example

```python report
client = MagicMock()
mocker.patch("app.service.fetch")
```

Use instead:

```python ok
client = create_autospec(Client)
mocker.patch("app.service.fetch", autospec=True)
```

## What it does

A `patch(..., spec=True)` gets an unsafe fix to `autospec=True`, because autospeccing checks call signatures that `spec`
does not, so a test that passed can start failing. No other call gets a fix: the spec is the real object the mock stands
in for, and that cannot be inferred from the call.

### Active on

- **[`Mock`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.Mock), `MagicMock`, `AsyncMock`,
  `NonCallableMock` and `NonCallableMagicMock` calls with no spec**: no positional argument, and no `spec` or `spec_set`
  keyword other than `None` or `False`. Mock classes match by the last part of their name, so `mock.MagicMock` and
  `mocker.MagicMock` are covered.

  - A bare name, such as `MagicMock()`

    ```python report
    client = MagicMock()
    ```

  - A dotted name, such as
    [`unittest.mock.NonCallableMock()`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.NonCallableMock)

    ```python report
    import unittest.mock

    client = unittest.mock.NonCallableMock()
    ```

- **[`patch`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.patch) calls with fewer than two
  positional arguments**: the second one is `new`, the replacement itself.

  ```python report
  from unittest.mock import patch

  patch("app.service.fetch")
  ```

- **[`patch.object`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.patch.object) calls with fewer
  than three**: the third one is `new`.

  ```python report
  mocker.patch.object(Client, "fetch")
  ```

- **Specs that do not check call signatures**: a `spec` or `spec_set` keyword or a positional spec, and a `wraps`
  keyword. `spec` limits the attribute names a mock has, but its methods still accept any arguments, so a call with the
  wrong arguments passes, and `wraps` delegates calls to the wrapped object. Use `create_autospec` or `autospec=True` to
  check signatures on the mock itself.

  ```python report file=tests/test_client.py
  from unittest.mock import MagicMock


  def test_fetch() -> None:
      client = MagicMock(spec=Client)
      client.fetch("/x")
  ```

  ```python report file=tests/test_wraps.py
  from unittest.mock import MagicMock


  def test_wraps(real_client: Client) -> None:
      client = MagicMock(wraps=real_client)
      client.fetch("/x")
  ```

  A `patch` with `spec=True` gets a fix to `autospec=True`, which is unsafe: it checks signatures, so a test that passed
  can start failing.

  ```python report file=tests/test_patch.py
  from unittest.mock import patch


  def test_fetch() -> None:
      with patch("app.service.fetch", spec=True):
          pass
  ```

  ```python fixed file=tests/test_patch.py
  from unittest.mock import patch


  def test_fetch() -> None:
      with patch("app.service.fetch", autospec=True):
          pass
  ```

`patch` is a common method name, so it matches only in these forms, and `patch.object` follows the same rule:

- a call that resolves to `unittest.mock.patch` or the `mock` backport's `patch`, including calls through an imported
  module;

  ```python report
  import unittest.mock

  unittest.mock.patch("app.service.fetch")
  ```

- an imported alias such as `mock_patch`, or a dotted name ending in the
  [pytest-mock](https://pytest-mock.readthedocs.io/en/latest/usage.html) fixture's `mocker.patch`.

  ```python report
  from unittest.mock import patch as mock_patch

  mock_patch("app.service.fetch")
  ```

### Exceptions

The rule skips the following calls.

- **An `autospec` or `new_callable` keyword on a `patch` or `patch.object` call**, unless its value is `None` or
  `False`: those values spell out the default, no spec. A mock constructor reads the keyword as an attribute, so it
  makes no spec.

  ```python ok
  mocker.patch("app.service.fetch", autospec=True)
  ```

  ```python report
  client = MagicMock(autospec=True)
  ```

  ```python report
  client = MagicMock(spec=None)
  ```

- **A `spec` or `spec_set` naming a class the module defines, when the test reads one of its members that autospeccing
  cannot see through the mock**: an instance attribute the constructor sets, or a property. The read counts when it is
  made on the variable the call is bound to, or on that variable's `return_value`, as in `client.session`. An attribute
  of the same name on another object does not count, and neither does a write. A spec for a class the module does not
  define, or one whose members the test does not read through the mock, is reported.

  ```python ok file=tests/test_session.py
  from unittest.mock import MagicMock


  class Client:
      def __init__(self) -> None:
          self.session = object()


  def test_session() -> None:
      client = MagicMock(spec=Client)
      assert client.session
  ```

  ```python report file=tests/test_fetch.py
  from unittest.mock import MagicMock


  class Client:
      def __init__(self) -> None:
          self.session = object()

      def fetch(self) -> None:
          pass


  def test_fetch() -> None:
      client = MagicMock(spec=Client)
      client.fetch()
  ```

- **A patch with `new`.** The rule skips any explicit replacement, including `new=DEFAULT`.

  ```python ok
  mocker.patch("app.service.fetch", new=fake_fetch)
  ```

- **A `**kwargs` argument**: it may carry any of these keywords.

  ```python ok
  client = MagicMock(**options)
  ```

These are not reported either:

- **`client.patch(...)` and `requests.patch(...)`**: `patch` is too common a method name, such as on HTTP clients, to
  match on the name alone.

  ```python ok
  client.patch("/users")
  requests.patch("https://example.com")
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#5734](https://github.com/astral-sh/ruff/issues/5734).
- [flake8-mock-spec](https://github.com/jdkandersson/flake8-mock-spec) reports a `Mock` built without a spec as
  [TMS010](https://github.com/jdkandersson/flake8-mock-spec), and a `patch` call without one as
  [TMS020](https://github.com/jdkandersson/flake8-mock-spec).
- [hacking](https://github.com/openstack/hacking) reports `mock.patch` calls without `autospec` as
  [H210](https://github.com/openstack/hacking/blob/master/hacking/checks/mock_checks.py). H210 accepts `autospec`,
  `new`, `new_callable`, `spec`, `spec_set` and `wraps`, as well as a positional replacement.
