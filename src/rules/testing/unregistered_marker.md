Flags a `pytest.mark.NAME` whose name the project's pytest configuration does not register under
[`markers`](https://docs.pytest.org/en/stable/reference/reference.html#confval-markers).

## Why is this bad?

By default, pytest warns about an unknown mark and lets collection continue. A test marked `@pytest.mark.slwo` runs
under `-m "not slow"`, and a `skipif` spelled wrong never skips. The
[`strict_markers`](https://docs.pytest.org/en/stable/how-to/mark.html#raising-errors-on-unknown-marks) option makes an
unknown mark a collection error. Registering the marks documents them in `pytest --markers` and lets this rule check
names without running tests.

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

The rule checks attributes such as `pytest.mark.slow`, including decorators, expressions in
`pytest.param(..., marks=...)` and assignments. It checks each supported configuration file in order until it finds one
that configures pytest.

1. `pytest.ini` or `.pytest.ini`, even when empty
1. `pyproject.toml` with `[tool.pytest]` or `[tool.pytest.ini_options]`
1. `tox.ini` with `[pytest]`
1. `setup.cfg` with `[tool:pytest]`

pytest 9 also reads
[`pytest.toml` and `.pytest.toml`](https://docs.pytest.org/en/stable/reference/customize.html#pytest-toml). simplint
skips those two formats.

### Exceptions

- **No pytest configuration**: without one the registered marks are unknown, so nothing is reported.

- **pytest's own marks**: `skip`, `skipif`, `xfail`, `parametrize`, `usefixtures`, `filterwarnings`, `tryfirst` and
  `trylast`.

- **Known plugin names.** The rule accepts these names even if the corresponding plugin is absent. Examples include
  `asyncio`, `anyio`, `trio`, `django_db`, `timeout`, `flaky`, `benchmark`, `order`, `dependency`, `xdist_group`,
  `httpx_mock`, `vcr`, `freeze_time`, `repeat`, `forked`, `subtests` and pytest-run-parallel's `thread_unsafe`.

- **Possible registrations in code.** The rule accepts a name that appears as a word in a project string, as in
  `config.addinivalue_line("markers", "network: needs internet access")`. It also accepts a mark assigned to a name,
  such as `C_EXT = pytest.mark.c_extension`, because a hook may register `C_EXT.name`.

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
