Flags imports of the top-level modules `gzip`, `bz2`, `lzma` and `zlib` when the target supports the
[`compression` package](https://docs.python.org/3/library/compression.html).

## Why is this bad?

Python 3.14 groups the four modules with [`compression.zstd`](https://docs.python.org/3/library/compression.zstd.html).
[PEP 784](https://peps.python.org/pep-0784/#other-compression-modules) recommends the names under `compression` for new
code. The top-level imports remain supported.

## Example

```toml config
[project]
requires-python = ">=3.14"
```

```python report
import gzip
from bz2 import BZ2File


def pack(path, data):
    with gzip.open(path, "wb") as handle:
        handle.write(data)
    return BZ2File(path + ".bz2", "wb")
```

Use instead:

```python ok
from compression import gzip
from compression.bz2 import BZ2File


def pack(path, data):
    with gzip.open(path, "wb") as handle:
        handle.write(data)
    return BZ2File(path + ".bz2", "wb")
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [tool.ruff]
  target-version = "py314"
  ```

  ```python report
  import gzip


  def read(path):
      with gzip.open(path) as handle:
          return handle.read()
  ```

## What it does

### Active on

- **Imports of `gzip`, `bz2`, `lzma` and `zlib`**, as `import gzip`, `import gzip as name` or `from gzip import name`,
  in any body but a `try` block. **Unsafe fix:** `import gzip` becomes `from compression import gzip`, and
  `from gzip import name` becomes `from compression.gzip import name`.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python report
  from gzip import GzipFile
  ```

  ```python fixed
  from compression.gzip import GzipFile
  ```

### Fix

The fix is unsafe. For example, `compression.gzip` imports the public names from `gzip` into a separate module. It has a
different `__name__` and lacks names outside `gzip.__all__`, including `gzip.READ`. Reassigning
[`gzip.open`](https://docs.python.org/3/library/gzip.html#gzip.open) after both modules have loaded leaves
`compression.gzip.open` pointing to the original function.

simplint leaves grouped imports such as `import os, gzip` for manual editing. It also withholds the fix for known
missing attributes, whether imported directly or accessed through the module, such as `gzip.READ` and
`zlib._ZlibDecompressor`.

The fix preserves an explicit `lazy` import on Python 3.15 and later.

```toml config
[project]
requires-python = ">=3.15"
```

```python report
lazy import gzip


def unpack(data):
    return gzip.decompress(data)
```

```python fixed
lazy from compression import gzip


def unpack(data):
    return gzip.decompress(data)
```

### Exceptions

- **Compatibility imports.** The rule skips imports directly in the body or clauses of a `try` statement, where code may
  fall back to the older names. It also skips files in a package named `compression`, which may itself implement those
  names.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python ok
  try:
      from compression import gzip
  except ImportError:
      import gzip
  ```

- **Targets before Python 3.14**: the `compression` package does not exist there.

  ```toml config
  [tool.ruff]
  target-version = "py313"
  ```

  ```python ok
  import gzip
  ```

## Other tools

- [flake8-tergeo](https://github.com/SAP/flake8-tergeo) reports the same imports as
  [FTP133](https://github.com/SAP/flake8-tergeo/blob/main/docs/checks.md#ftp133), with no fix.
