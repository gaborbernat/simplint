Flags imports of [`gzip`](https://docs.python.org/3/library/gzip.html),
[`bz2`](https://docs.python.org/3/library/bz2.html), [`lzma`](https://docs.python.org/3/library/lzma.html) and
[`zlib`](https://docs.python.org/3/library/zlib.html), which the
[`compression`](https://docs.python.org/3/library/compression.html) package names from Python 3.14
([PEP 784](https://peps.python.org/pep-0784/)).

## Why is this bad?

Python 3.14 adds the [`compression`](https://docs.python.org/3/library/compression.html) package, with
`compression.gzip`, `compression.bz2`, `compression.lzma` and `compression.zlib` beside
[`compression.zstd`](https://docs.python.org/3/library/compression.zstd.html).
[PEP 784](https://peps.python.org/pep-0784/#other-compression-modules) makes these the preferred import names. One
package puts the modules where readers look for them, and its names avoid clashes with PyPI projects that already hold
bare names such as `zstd`. The old top-level modules are
[not deprecated](https://peps.python.org/pep-0784/#backwards-compatibility) and keep working without a warning, so the
rule is about consistency with the new names.

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

The fix is unsafe, because `compression.gzip` is not an alias of `gzip`. It is a module that runs `from gzip import *`,
so it lacks the names outside `__all__`, such as `gzip.READ`, and it is a different module object: a patch of
[`gzip.open`](https://docs.python.org/3/library/gzip.html#gzip.open) does not reach a call through `compression.gzip`. A
rewritten import also changes the module's `__name__`.

No fix is offered for a statement that binds other names as well, such as `import os, gzip`. Nor is one offered when the
import, or a read through the name it binds, takes a name the new module lacks, such as `gzip.READ` or
`zlib._ZlibDecompressor`.

A `lazy import` keeps its `lazy` keyword in the rewritten `from` import, so the import stays lazy:

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

- **Imports in a `try` block**: such a block is a fallback for the Pythons that lack the new names.

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

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
- [flake8-tergeo](https://github.com/SAP/flake8-tergeo) reports the same imports as
  [FTP133](https://github.com/SAP/flake8-tergeo/blob/main/docs/checks.md#ftp133), with no fix.
