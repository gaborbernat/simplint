## What it does

Checks for attributes set on a mock right after it is created. The mock must be assigned to a plain name, with or
without an annotation, from a call to [`Mock`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.Mock),
`MagicMock`, `AsyncMock`, `NonCallableMock`, `NonCallableMagicMock` or
[`create_autospec`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.create_autospec). The call
matches by the last part of its name, so `mocker.MagicMock()` counts too. Only the run of `mock.attr = value` statements
directly after the creation is reported. The run stops at:

- any other statement, including `mock.child.attr = value`;
- an attribute the constructor treats specially: `spec`, `spec_set`, `wraps`, `name`, `parent`, `unsafe`, `instance`,
  `_name` or `_parent`;
- an attribute already set earlier in the run;
- a value that refers to the mock itself.

## Why is this bad?

Setting attributes after construction spreads the mock's configuration over several statements. The mock constructors
take any attribute as a keyword argument and set it on the new mock, so the whole setup fits in one call.

## Example

```python
fetch = create_autospec(fetch_user)
fetch.return_value = user
fetch.side_effect = None
```

Use instead:

```python
fetch = create_autospec(fetch_user, return_value=user, side_effect=None)
```

## Fix safety

The fix moves the assignments into the constructor call as keyword arguments and deletes them. An assignment to a
keyword the call already passes replaces that keyword's value. No fix is offered when the call has a `**kwargs`
argument, an assignment shares a line with another statement, or a comment sits on its lines.

The fix is safe, except for a
[`create_autospec`](https://docs.python.org/3/library/unittest.mock.html#unittest.mock.create_autospec) call that sets
an attribute other than `return_value` or `side_effect`. When the spec is a function, `create_autospec` returns a
wrapper function and hands the other keywords to the mock inside it, so the test no longer sees them on the object it
holds.
