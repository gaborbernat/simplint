## What it does

Checks for mocks and patches created without a spec:

- [`Mock`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.Mock), `MagicMock`, `AsyncMock`,
  `NonCallableMock` and `NonCallableMagicMock` calls with no positional argument;
- [`patch`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.patch) calls with fewer than two
  positional arguments;
- [`patch.object`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.patch.object) calls with fewer
  than three.

A `spec`, `spec_set`, `wraps`, `autospec` or `new_callable` keyword counts as a spec unless its value is `None` or
`False`. A `new` keyword always counts, since its value replaces the target and no mock is created, and so does a
`**kwargs` argument. Mock classes match by the last part of their name, so `mock.MagicMock` and `mocker.MagicMock` are
covered.

`patch` is a common method name, so it matches only in these forms:

- a dotted name ending in `mock.patch` or `mocker.patch`, such as `unittest.mock.patch` or the
  [pytest-mock](https://pytest-mock.readthedocs.io/en/latest/usage.html) fixture's `mocker.patch`;
- `patch` imported from `unittest.mock` or `mock`, under any alias.

`client.patch(...)` and `requests.patch(...)` are not reported. `patch.object` follows the same rule.

## Why is this bad?

A mock without a spec accepts any attribute and any call signature, so a test keeps passing after the real code renames
a method or changes its parameters. [Autospeccing](https://docs.python.org/3/library/unittest.mock.html#autospeccing)
makes the mock fail the same way the real object would.

## Example

```python
client = MagicMock()
mocker.patch("app.service.fetch")
```

Use instead:

```python
client = create_autospec(Client)
mocker.patch("app.service.fetch", autospec=True)
```

## Fix safety

No fix is offered: the spec is the real object the mock stands in for, and that cannot be inferred from the call.
