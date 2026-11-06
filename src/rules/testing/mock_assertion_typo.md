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

- **Names that Ruff's PGH005 reports**: [PGH005](https://docs.astral.sh/ruff/rules/invalid-mock-access/) reports these
  names, so this rule leaves them to Ruff only when the project's Ruff configuration selects PGH005. With Ruff's preview
  mode off, the test of an `assert` is the only place, bare or called. With preview on, a name on a line of its own is
  one too. The set follows the project's preview setting. The operand of a `not` or `and`, or a comparison, is not
  checked by Ruff, so this rule still reports a name there.

  ```toml config
  [tool.ruff.lint]
  select = ["PGH005"]
  ```

  ```python ok
  def test_save(save: MagicMock) -> None:
      run_save()
      assert save.not_called()
  ```

### Fix

The fix renames the attribute to the assertion it stands for. **Unsafe fix:** the call becomes a check, so a test that
passed without checking may now fail. Every report offers the fix, since each one names a single assertion to rename to.

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

## Ruff overlap

There is no overlap. Ruff's [invalid-mock-access](https://docs.astral.sh/ruff/rules/invalid-mock-access/) (PGH005)
reports the prefix-less assertion names in the test of an `assert`, bare or called, such as `assert mock.not_called()`,
and with Ruff's preview mode on, also those names on a line of their own. This rule follows the same preview setting: it
leaves those names to Ruff only when the project's Ruff configuration selects PGH005, and reports the rest, which are
operands of `not` and `and`, comparisons, names in call arguments, and every misspelling, since Ruff does not report
those. Ruff's own suppressions of it, through `per-file-ignores` or `noqa`, are respected too.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** released rule
  [PGH005](https://docs.astral.sh/ruff/rules/invalid-mock-access/).
- [flake8-mock](https://github.com/zupo/flake8-mock) reports misspelled mock assertions, such as `called_once_with`, as
  [M001](https://github.com/zupo/flake8-mock), with no fix. This rule's fix renames them to the assertion they stand
  for.
- [Ruff](https://docs.astral.sh/ruff/rules/invalid-mock-access/) reports the names in the test of an `assert`, such as
  `assert mock.not_called()`, as [PGH005](https://docs.astral.sh/ruff/rules/invalid-mock-access/), with no fix. This
  rule leaves those to it; see Ruff overlap.
