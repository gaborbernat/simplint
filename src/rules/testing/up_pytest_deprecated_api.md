Flags obsolete pytest hooks and arguments, marks on fixtures, and fixtures that use the legacy `py.path` API.

## Why is this bad?

pytest 8
[removed nose-style hooks](https://docs.pytest.org/en/stable/deprecations.html#support-for-tests-written-for-nose). A
test that depended on `setup` may now run without its setup.
[Marks on fixtures](https://docs.pytest.org/en/stable/deprecations.html#applying-a-mark-to-a-fixture-function) never
applied to the tests requesting them, and pytest 9 rejects them.
[Removed arguments](https://docs.pytest.org/en/stable/deprecations.html#removed-features-and-breaking-changes) can raise
[`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError). The legacy fixtures still work when their
plugins are available. Use `tmp_path`, `tmp_path_factory` and `pytester` for their modern interfaces.

## Example

```python report file=tests/test_config.py
import pytest


def test_missing(tmpdir):
    if not tmpdir.join("config.toml").exists():
        pytest.skip(msg="no config")
```

Use instead:

```python ok file=tests/test_config_current.py
import pytest


def test_missing(tmp_path):
    if not (tmp_path / "config.toml").exists():
        pytest.skip(reason="no config")
```

## What it does

### Active on

The rule checks [test modules](SMP801.html).

- **Nose-style hooks**: a module-level `setup` or `teardown` function, and a `setup` or `teardown` method of a test
  class that does not derive from
  [`unittest.TestCase`](https://docs.python.org/3/library/unittest.html#unittest.TestCase).

  ```python report file=tests/test_nose.py
  class TestStore:
      def setup(self):
          self.store = Store()

      def test_empty(self):
          assert not self.store
  ```

- **Marks on fixtures**: any `pytest.mark` decorator on a `pytest.fixture` function, other than `usefixtures`.

  ```python report file=tests/test_marked_fixture.py
  import pytest


  @pytest.mark.slow
  @pytest.fixture
  def database():
      return connect()
  ```

- **Legacy fixtures.** In tests and fixtures, use `tmp_path` for a temporary directory, `tmp_path_factory` to create
  directories, and `pytester` to run pytest in a test. These replace `tmpdir`, `tmpdir_factory` and `testdir`,
  respectively.

- **Removed arguments**: `msg=` to `pytest.fail`, `pytest.skip` or `pytest.exit`, `message=` to `pytest.raises`, and
  `pytest.warns(None)`.

### Exceptions

- **Hooks the module calls itself or decorates**: a `setup` that the module names more than once may be a helper. The
  rule also skips decorated hooks and hooks in `conftest.py`. The same exceptions apply to `teardown`.

  ```python ok file=tests/test_called_setup.py
  def setup():
      return {"debug": True}


  def test_debug():
      assert setup()["debug"]
  ```

- **Modules that test the `legacypath` plugin**: a module that imports it requests its fixtures on purpose.

- **Calls inside a `with pytest.raises(...)` body.** The test may be checking that pytest rejects an obsolete argument.

- **`yield_fixture`**: Ruff's [pytest-yield-fixture](https://docs.astral.sh/ruff/rules/pytest-deprecated-yield-fixture/)
  (PT020) reports it.

### Fix

When the call passes `msg=` without `reason=`, the safe fix renames the keyword and preserves its value.

```python report file=tests/test_skip.py
import pytest


def test_later():
    pytest.skip(msg="not ready")
```

```python fixed file=tests/test_skip.py
import pytest


def test_later():
    pytest.skip(reason="not ready")
```

The unsafe fix renames a class hook to `setup_method` or `teardown_method`, and a module hook to `setup_module` or
`teardown_module`. pytest will start running the hook, which changes what the tests see.

```python report file=tests/test_nose_fixed.py
def setup():
    print("start")
```

```python fixed file=tests/test_nose_fixed.py
def setup_module():
    print("start")
```

Update the other cases by hand. For example, a migration from `tmpdir` to `tmp_path` may also require changing `.join()`
calls to `/`.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rules
  [PT020](https://docs.astral.sh/ruff/rules/pytest-deprecated-yield-fixture/), which reports `pytest.yield_fixture`, and
  [PT025](https://docs.astral.sh/ruff/rules/pytest-erroneous-use-fixtures-on-fixture/), which reports `usefixtures` on a
  fixture; this rule leaves both to Ruff.
