Flags rows of a literal `pytest.mark.parametrize` list that are not `pytest.param(..., id=...)`.

## Why is this bad?

Without an id,
[pytest builds one](https://docs.pytest.org/en/stable/example/parametrize.html#different-options-for-test-ids) from the
values, such as `test_lower[FALSE-false]`, or from the argument name and position, such as `values0`, for values it does
not print. A written id says what the case covers and stays the same when the values change.

## Example

```python report file=tests/test_total.py
import pytest


@pytest.mark.parametrize(
    ("values", "expected"),
    [
        ([1, 2], 3),
        ([], 0),
    ],
)
def test_total(values: list[int], expected: int) -> None:
    assert sum(values) == expected
```

Use instead:

```python ok file=tests/test_total.py
import pytest


@pytest.mark.parametrize(
    ("values", "expected"),
    [
        pytest.param([1, 2], 3, id="pair"),
        pytest.param([], 0, id="empty"),
    ],
)
def test_total(values: list[int], expected: int) -> None:
    assert sum(values) == expected
```

## What it does

A row without an id gets no fix: a meaningful id describes what the case tests, which cannot be derived from its values.
An `ids=` list gets one that moves each id into its row.

### Active on

- **Rows in a literal [`pytest.mark.parametrize`](https://docs.pytest.org/en/stable/how-to/parametrize.html) list or
  tuple that are not
  [`pytest.param(..., id=...)`](https://docs.pytest.org/en/stable/reference/reference.html#pytest-param)**: without one,
  pytest makes up the id.

  ```python report
  import pytest


  @pytest.mark.parametrize(
      ("values", "expected"),
      [
          ([1, 2], 3),
          ([], 0),
      ],
  )
  def test_total(values: list[int], expected: int) -> None:
      assert sum(values) == expected
  ```

- **Rows whose `id` is `None`**: `id=None` counts as no id.

  ```python report
  import pytest


  @pytest.mark.parametrize(("values", "expected"), [pytest.param([1, 2], 3, id=None)])
  def test_total(values: list[int], expected: int) -> None:
      assert sum(values) == expected
  ```

- **Names bound to a `pytest.param(...)` without an id**: a row name bound once to such a call is reported where the
  call is written. A name bound more than once, such as by both branches of an `if`, is reported where the row is
  written.

  ```python report
  import pytest

  CASE = pytest.param([1, 2], 3)


  @pytest.mark.parametrize(("values", "expected"), [CASE])
  def test_total(values: list[int], expected: int) -> None:
      assert sum(values) == expected
  ```

- **Rows whose id appears more than once in the list**: pytest suffixes the shared id, such as `select0` and `select1`,
  so the ids say nothing about the cases. Numbers count by value, so `0x10` and `16` repeat each other.

  ```python report
  import pytest


  @pytest.mark.parametrize("tag", ["select", "input", "select"])
  def test_tag(tag: str) -> None:
      assert tag
  ```

- **`ids=` lists of string literals**: the list names each case apart from its row, so the two drift apart when a row is
  added or moved. The fix wraps each row in `pytest.param(..., id=...)` and drops `ids=`, when the list has one id per
  row and each row is a literal: a tuple or list with one value per name, or a literal value for a single name. A row
  that is a name, a call or already a `pytest.param(...)`, or a list of a different length, is reported without a fix. A
  name bound once at module level to such a list is reported without a fix too. The fix is unsafe when a comment sits in
  the rows or the `ids=` argument, since the rewrite drops it.

  ```python report
  import pytest


  @pytest.mark.parametrize(("values", "expected"), [([1, 2], 3), ([], 0)], ids=["pair", "empty"])
  def test_total(values: list[int], expected: int) -> None:
      assert sum(values) == expected
  ```

  ```python fixed
  import pytest


  @pytest.mark.parametrize(("values", "expected"), [pytest.param([1, 2], 3, id="pair"), pytest.param([], 0, id="empty")])
  def test_total(values: list[int], expected: int) -> None:
      assert sum(values) == expected
  ```

### Exceptions

- **Rows that pytest uses as their id**: a string or bytes of printable ASCII without a backslash, a number, `True`,
  `False` or `None` is its own id, such as `test_tag[select]`. A row unpacked into several arguments whose values are
  all such scalars gets them joined with `-`, such as `test_lower[FALSE-false]`. Wrapping either in
  `pytest.param(..., id=...)` would only repeat it.

  ```python ok
  import pytest


  @pytest.mark.parametrize("tag", ["select", "input"])
  def test_tag(tag: str) -> None:
      assert tag


  @pytest.mark.parametrize(("raw", "expected"), [("FALSE", "false"), ("True", "true")])
  def test_lower(raw: str, expected: str) -> None:
      assert raw.lower() == expected
  ```

- **Starred values the rule cannot read**, such as a generator, `*(pytest.param(...) for ...)`: the rows they unpack are
  not known to the rule. The same holds for a name imported from another module, which is not followed, a name bound
  more than once, and a starred element inside a list that is spread. A starred list or tuple, written inline or a name
  bound once to one, has its rows checked where that list is written.

  ```python ok
  import pytest


  @pytest.mark.parametrize("kind", [*(pytest.param(kind, id=kind) for kind in ("a", "b")), "c"])
  def test_kind(kind: str) -> None:
      assert kind
  ```

- **Row names with an id, imports, and names the rule cannot read**: a name one of whose bindings is a
  [`pytest.param(..., id=...)`](https://docs.pytest.org/en/stable/reference/reference.html#pytest-param) carries the id,
  so the rule leaves it. So does a name imported from another module, which is not followed, and a name bound to a value
  the rule cannot read, such as a function parameter or an unpacked tuple.

  ```python ok
  import pytest

  CASE = pytest.param([1, 2], 3, id="pair")


  @pytest.mark.parametrize(("values", "expected"), [CASE])
  def test_total(values: list[int], expected: int) -> None:
      assert sum(values) == expected
  ```

- **Calls to a helper whose last statement returns an id**: a function the module defines once carries the id when its
  last statement is a `return` of
  [`pytest.param(..., id=...)`](https://docs.pytest.org/en/stable/reference/reference.html#pytest-param) or a `raise`,
  or an `if`, `match`, `try` or `with` whose branches all end that way, and every `return` in it gives a `pytest.param`
  with an id. Loops don't count, and neither does a `with` around a manager that can swallow exceptions:
  [`contextlib.suppress`](https://docs.python.org/3/library/contextlib.html#contextlib.suppress), `pytest.raises`,
  [`contextlib.ExitStack`](https://docs.python.org/3/library/contextlib.html#contextlib.ExitStack) and
  [`contextlib.AsyncExitStack`](https://docs.python.org/3/library/contextlib.html#contextlib.AsyncExitStack), or any
  manager that is not a call, such as a name. A helper that returns a tuple or a row without an id, an `async` helper,
  one that yields, one whose last statement does not end that way, or a call to a function the module does not define is
  reported where the call is written. Decorated helpers stay reported, except
  [`functools.cache`](https://docs.python.org/3/library/functools.html#functools.cache) and
  [`functools.lru_cache`](https://docs.python.org/3/library/functools.html#functools.lru_cache), and so do lambdas.

  ```python ok
  import pytest


  def _case(values: list[int], expected: int) -> pytest.ParameterSet:
      return pytest.param(values, expected, id=f"{len(values)}-values")


  @pytest.mark.parametrize(("values", "expected"), [_case([1, 2], 3), _case([], 0)])
  def test_total(values: list[int], expected: int) -> None:
      assert sum(values) == expected
  ```

- **Rows built elsewhere**, such as a module constant passed as the argument values: the decorator holds no rows to
  check.

  ```python ok
  import pytest

  CASES = [([1, 2], 3), ([], 0)]


  @pytest.mark.parametrize(("values", "expected"), CASES)
  def test_total(values: list[int], expected: int) -> None:
      assert sum(values) == expected
  ```

- **`ids=` that generates the ids**: a callable, such as a function or a lambda, or an expression that is not a list of
  string literals, makes the ids from the values, so they cannot drift from the rows.

  ```python ok
  import pytest


  @pytest.mark.parametrize(("values", "expected"), [([1, 2], 3), ([], 0)], ids=repr)
  def test_total(values: list[int], expected: int) -> None:
      assert sum(values) == expected
  ```

- **`ids=` names the rule cannot read**: a name imported from another module, bound inside a function, or bound more
  than once is accepted, since the rule cannot see its list.

  ```python ok
  import pytest

  from cases import SHARED_IDS


  @pytest.mark.parametrize("value", [1, 2], ids=SHARED_IDS)
  def test_value(value: int) -> None:
      assert value
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open pull request
  [#22136](https://github.com/astral-sh/ruff/pull/22136) would flag boolean-only parametrize lists without `ids=`, a
  subset of this rule. Open issue [#14994](https://github.com/astral-sh/ruff/issues/14994) asks the opposite, replacing
  `ids=` with `pytest.param(id=...)`.
