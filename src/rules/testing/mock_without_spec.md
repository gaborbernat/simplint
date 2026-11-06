Flags mocks and patches created without a spec.

## Why is this bad?

A mock without a spec accepts any attribute and any call signature, so a test keeps passing after the real code renames
a method or changes its parameters. [Autospeccing](https://docs.python.org/3/library/unittest.mock.html#autospeccing)
makes the mock fail the same way the real object would.

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

No fix is offered: the spec is the real object the mock stands in for, and that cannot be inferred from the call.

### Active on

- **[`Mock`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.Mock), `MagicMock`, `AsyncMock`,
  `NonCallableMock` and `NonCallableMagicMock` calls with no positional argument**: the first positional argument is the
  spec. Mock classes match by the last part of their name, so `mock.MagicMock` and `mocker.MagicMock` are covered.

  - A bare name, such as `MagicMock()`

    ```python report
    client = MagicMock()
    ```

  - A dotted name, such as `unittest.mock.NonCallableMock()`

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

`patch` is a common method name, so it matches only in these forms, and `patch.object` follows the same rule:

- a dotted name ending in `mock.patch` or `mocker.patch`, such as `unittest.mock.patch` or the
  [pytest-mock](https://pytest-mock.readthedocs.io/en/latest/usage.html) fixture's `mocker.patch`;
- `patch` imported from `unittest.mock` or `mock`, under any alias.

### Exceptions

These count as a spec:

- **A `spec`, `spec_set`, `wraps`, `autospec` or `new_callable` keyword**, unless its value is `None` or `False`: those
  values spell out the default, no spec.

  ```python ok
  client = MagicMock(spec=Client)
  ```

  ```python report
  client = MagicMock(spec=None)
  ```

- **A `new` keyword, always**: its value replaces the target, so no mock is created, whatever the value.

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

- [flake8-mock-spec](https://github.com/jdkandersson/flake8-mock-spec) reports a `Mock` built without a spec as
  [TMS010](https://github.com/jdkandersson/flake8-mock-spec), and a `patch` call without one as
  [TMS020](https://github.com/jdkandersson/flake8-mock-spec).
- [hacking](https://github.com/openstack/hacking) reports `mock.patch` calls without `autospec` as
  [H210](https://github.com/openstack/hacking/blob/master/hacking/checks/mock_checks.py). It accepts only `autospec`,
  where this rule also accepts `spec`, `spec_set` and `wraps`.
