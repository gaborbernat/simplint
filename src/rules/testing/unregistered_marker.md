Flags a `pytest.mark.NAME` whose name the project's pytest configuration does not register under
[`markers`](https://docs.pytest.org/en/stable/reference/reference.html#confval-markers).

## Why is this bad?

pytest accepts any attribute of `pytest.mark`, so a misspelled mark is a new mark rather than an error. A test marked
`@pytest.mark.slwo` runs under `-m "not slow"`, and a `skipif` spelled wrong never skips. pytest only emits a
`PytestUnknownMarkWarning`, unless the
[`strict_markers`](https://docs.pytest.org/en/stable/how-to/mark.html#raising-errors-on-unknown-marks) option turns it
into an error at collection. Registering the marks documents them in `pytest --markers` and lets this rule check every
use without running the tests.

## Example

```toml config
[tool.pytest.ini_options]
markers = ["slow: tests that take more than a second"]
```

```python report file=tests/test_import.py
import pytest


@pytest.mark.slwo
def test_full_import():
    assert import_all()
```

## What it does

### Active on

- **Every `pytest.mark` attribute** in the module, as a decorator, in `pytest.param(..., marks=...)`, or bound to a
  name, when the project configures pytest. The configuration is read from the first of `pytest.ini`, `.pytest.ini`,
  `pyproject.toml` (`[tool.pytest]` or `[tool.pytest.ini_options]`), `tox.ini` (`[pytest]`) and `setup.cfg`
  (`[tool:pytest]`) that configures pytest, as pytest reads it.

### Exceptions

- **No pytest configuration**: without one the registered marks are unknown, so nothing is reported.

- **pytest's own marks**: `skip`, `skipif`, `xfail`, `parametrize`, `usefixtures`, `filterwarnings`, `tryfirst` and
  `trylast`.

- **Marks common plugins register**, such as `asyncio`, `anyio`, `trio`, `django_db`, `timeout`, `flaky`, `benchmark`,
  `order`, `dependency`, `xdist_group`, `httpx_mock`, `vcr`, `freeze_time`, `repeat`, `forked`, `subtests` and
  pytest-run-parallel's `thread_unsafe`.

- **Marks registered in code**: a mark whose name appears in any string of the project, as
  `config.addinivalue_line("markers", "slow: ...")` in a `pytest_configure` hook spells it, and a mark bound to a name,
  as `C_EXT = pytest.mark.c_extension`, which the hook registers through `C_EXT.name`.

  ```toml config
  [tool.pytest.ini_options]
  markers = ["slow"]
  ```

  ```python ok file=tests/conftest.py
  def pytest_configure(config):
      config.addinivalue_line("markers", "network: needs internet access")
  ```

  ```python ok file=tests/test_fetch.py
  import pytest


  @pytest.mark.network
  def test_fetch():
      assert fetch("https://example.com")
  ```

### Fix

No fix is offered, as the mark may be a typo of a registered one or a new mark to register.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
