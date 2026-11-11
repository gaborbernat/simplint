Flags attribute names that are mock assertion methods written without their `assert_` prefix, or misspelled.

## Why is this bad?

`unittest.mock` raises `AttributeError` for an unknown name that starts with `assert`, `assret`, `asert`, `aseert` or
`assrt`, so a misspelled assertion with one of those prefixes fails the test. Other names are ordinary attributes to the
mock: the call returns a child mock and checks nothing. Python 3.12 added the names of the `Mock` assertions without
their prefix, such as `called_once_with`, to the names a mock rejects, but not those of the `AsyncMock` assertions, such
as `awaited_once_with`. So `called_once_with` passes silently on Python 3.11 and older, and `awaited_once_with` or a
misspelling such as `asssert_called_once` passes silently on every version. The lint reports these before the test runs.

## Example

```python report
def test_save(save: MagicMock) -> None:
    run_save()
    save.called_once_with("out.json")
```

Use instead:

```python ok
def test_save(save: MagicMock) -> None:
    run_save()
    save.assert_called_once_with("out.json")
```

## What it does

Reports an attribute, read or called, whose name is one of these:

- an assertion method without its `assert_` prefix, such as `called_with`, `has_calls` or `not_called`, but not
  `called`, which every mock has as an attribute;
- an assertion method with a misspelled name, such as `asssert_called_once` or `assetr_called_with`, that is within two
  edits of exactly one assertion method. Only names that start with `as` are compared, since a misspelled prefix keeps
  those two letters, and not those that start with a prefix the mock rejects.

### Active on

- **Assertion methods without their prefix**: the name is the assertion's name minus `assert_`.

  ```python report
  mock.called_once()
  ```

  ```python report
  mock.has_calls([call(1)])
  ```

- **Misspelled assertion methods**: the nearest assertion method is the one meant, when it is the only nearest.

  ```python report
  mock.asssert_called_once()
  ```

  ```python report
  mock.assetr_called_with(1)
  ```

### Exceptions

- **`called`**: a mock attribute that records whether it was called, not an assertion.

  ```python ok
  assert mock.called
  ```

- **Names that are not close to an assertion**: `assert_equal` on a helper or a numeric library is left alone.

  ```python ok
  numbers.assert_equal(1)
  ```

- **Names with a prefix the mock rejects**: a mock raises `AttributeError` for `assert_called_ones_with`, so the test
  already fails, and the same names are methods of other classes, such as `assert_failed` on a test helper.

  ```python ok
  outcome.assert_failed()
  ```

- **Names the file defines or the project assigns**: such a name is a class's own attribute, not a mock method. A name
  the file defines as a function, or the project sets or deletes as an attribute anywhere, is left alone.

  ```python ok
  class Spec:
      def called_once(self) -> None: ...


  def test_spec(mock: MagicMock) -> None:
      mock.called_once()
  ```

- **Code that expects the call to fail**: a `with` block of `pytest.raises` or `assertRaises...` checks the error, so
  the misspelled name is the point.

  ```python ok
  def test_unsafe_name(mock: MagicMock) -> None:
      with pytest.raises(AttributeError):
          mock.called_once()
  ```

- **Names that Ruff's PGH005 reports and this rule cannot fix**:
  [PGH005](https://docs.astral.sh/ruff/rules/invalid-mock-access/) reports these names in the test of an `assert` and on
  a line of their own, bare or called, so this rule leaves them to Ruff when it has no fix for them. The operand of a
  `not` or `and`, or a comparison, is not checked by Ruff, so this rule still reports a name there.

  ```python ok
  def test_save(save: MagicMock) -> None:
      run_save()
      assert save.not_called
  ```

### Fix

The fix renames the attribute to the assertion it stands for. **Unsafe fix:** the call becomes a check, so a test that
passed without checking may now fail.

```python report
def test_save(save: MagicMock) -> None:
    run_save()
    save.called_once_with("out.json")
```

```python fixed
def test_save(save: MagicMock) -> None:
    run_save()
    save.assert_called_once_with("out.json")
```

An assertion method returns `None`, so when the call is the whole test of an `assert` without a message, the fix drops
the `assert` and any parentheses around the call.

```toml config
[tool.ruff.lint]
select = []
```

```python report
def test_save(save: MagicMock) -> None:
    run_save()
    # fmt: off
    assert (save.not_called())
    # fmt: on
```

```python fixed
def test_save(save: MagicMock) -> None:
    run_save()
    # fmt: off
    save.assert_not_called()
    # fmt: on
```

```toml config
[tool.ruff.lint]
select = []
```

```python report
def test_save(save: MagicMock) -> None:
    run_save()
    assert save.not_called()
```

```python fixed
def test_save(save: MagicMock) -> None:
    run_save()
    save.assert_not_called()
```

Only a call on a line of its own, or the whole test of such an `assert`, gets the fix. Renamed anywhere else, the name
would be a bound method that is always true, or the `None` the method returns, which the code then goes on to use.

```python report
def test_save(save: MagicMock) -> None:
    run_save()
    assert not save.called_once
```

A comment in the text the fix drops, such as one inside the parentheses of the test, keeps the call unfixed, so the
comment is not lost. PGH005 reports such a name without a fix, so this rule leaves it to Ruff. A comment inside the call
does not block the fix.

```python ok
def test_save(save: MagicMock) -> None:
    run_save()
    assert (  # the writer must be reached exactly once
        save.called_once_with("out.json")
    )
```

## Ruff overlap

Ruff's [invalid-mock-access](https://docs.astral.sh/ruff/rules/invalid-mock-access/) (PGH005) reports the prefix-less
assertion names in the test of an `assert`, bare or called, such as `assert mock.not_called()`, and the names on a line
of their own. This rule leaves those names to Ruff when it has no fix for them, and reports the rest: the names it can
rename, such as `assert mock.called_once_with()`, since PGH005 offers no fix, the operands of `not` and `and`,
comparisons, names in call arguments, and every misspelling, which Ruff does not report.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [PGH005](https://docs.astral.sh/ruff/rules/invalid-mock-access/), which reports an assertion name missing its
  `assert_` prefix and which this rule leaves to it; this rule covers misspellings and the rest.
- [flake8-mock](https://github.com/zupo/flake8-mock) reports misspelled mock assertions, such as `called_once_with`, as
  [M001](https://github.com/zupo/flake8-mock), with no fix. This rule's fix renames them to the assertion they stand
  for.
