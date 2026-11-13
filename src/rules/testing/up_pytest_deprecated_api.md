Flags pytest APIs that pytest removed or no longer honors: nose-style `setup` and `teardown` hooks, marks on fixtures,
the `py.path` fixtures `tmpdir`, `tmpdir_factory` and `testdir`, the `msg` argument of `pytest.fail`, `pytest.skip` and
`pytest.exit`, `pytest.raises(message=...)`, and `pytest.warns(None)`.

## Why is this bad?

Each of these either does nothing on a current pytest or fails there. pytest 8
[dropped nose support](https://docs.pytest.org/en/stable/deprecations.html#support-for-tests-written-for-nose), so a
`setup` method silently stops running, and the tests that relied on it fail or, worse, pass against stale state.
[A mark on a fixture](https://docs.pytest.org/en/stable/deprecations.html#applying-a-mark-to-a-fixture-function) never
applied to the tests, and pytest 9 rejects it. The
[removed arguments](https://docs.pytest.org/en/stable/deprecations.html#removed-features-and-breaking-changes) raise a
`TypeError`. The `py.path` fixtures live on only in pytest's `legacypath` plugin; the
[pathlib fixtures](https://docs.pytest.org/en/stable/how-to/tmp_path.html) replace them.

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

- **Nose-style hooks**: a module-level `setup` or `teardown` function, and a `setup` or `teardown` method of a test
  class that does not derive from `unittest.TestCase`.

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

- **`py.path` fixtures**: a parameter named `tmpdir`, `tmpdir_factory` or `testdir` of a test or a fixture.

- **Removed arguments**: `msg=` to `pytest.fail`, `pytest.skip` or `pytest.exit`, `message=` to `pytest.raises`, and
  `pytest.warns(None)`.

### Exceptions

- **Hooks the module calls itself or decorates**: a `setup` that the module also calls, a decorated `setup`, such as a
  fixture of that name, and any `setup` in a `conftest.py` is not a nose hook.

  ```python ok file=tests/test_called_setup.py
  def setup():
      return {"debug": True}


  def test_debug():
      assert setup()["debug"]
  ```

- **Modules that test the `legacypath` plugin**: a module that imports it requests its fixtures on purpose.

- **Calls inside `pytest.raises`**: a test that checks pytest rejects the removed argument.

- **`yield_fixture`**: Ruff's [pytest-yield-fixture](https://docs.astral.sh/ruff/rules/pytest-deprecated-yield-fixture/)
  (PT020) reports it.

### Fix

The `msg` argument is renamed to `reason`, which takes the same text.

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

A nose hook is renamed to the pytest hook that runs at the same point: `setup_method` and `teardown_method` in a class,
`setup_module` and `teardown_module` at module level. **Unsafe fix:** pytest starts running the hook again, which
changes what the tests see.

```python report file=tests/test_nose_fixed.py
def setup():
    print("start")
```

```python fixed file=tests/test_nose_fixed.py
def setup_module():
    print("start")
```

The other cases are reported without a fix, since moving from `py.path` to `pathlib` changes every use of the path.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rules
  [PT020](https://docs.astral.sh/ruff/rules/pytest-deprecated-yield-fixture/), which reports `pytest.yield_fixture`, and
  [PT025](https://docs.astral.sh/ruff/rules/pytest-erroneous-use-fixtures-on-fixture/), which reports `usefixtures` on a
  fixture; this rule leaves both to Ruff.
