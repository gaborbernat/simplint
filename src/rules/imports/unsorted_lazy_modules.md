Flags a [`__lazy_modules__`](https://docs.python.org/3.15/reference/simple_stmts.html#compatibility-via-lazy-modules)
list whose entries are not sorted.

## Why is this bad?

The list grows with every import a module defers. Sorted, it reads like the import block below it, a missing or
duplicated module stands out, and two branches that each add a module merge without a conflict over where it goes.

## Example

```python report
__lazy_modules__ = ["pathlib", "json"]

import json
import pathlib
```

```python fixed
__lazy_modules__ = ["json", "pathlib"]

import json
import pathlib
```

## What it does

### Active on

- **A module-level `__lazy_modules__` assigned a list, tuple or set of literal entries.** Plain names come first in
  alphabetical order, then the `f"{__spec__.parent}..."` entries that name a module relative to the file's package, as a
  line sort in an editor orders them and [flake8-lazy](https://github.com/henryiii/flake8-lazy) expects.

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/helper.py
  VALUE = 1
  ```

  ```python report file=pkg/consumer.py
  __lazy_modules__ = [f"{__spec__.parent}.helper", "json"]

  import json

  from .helper import VALUE
  ```

  ```python fixed file=pkg/consumer.py
  __lazy_modules__ = ["json", f"{__spec__.parent}.helper"]

  import json

  from .helper import VALUE
  ```

### Exceptions

- **A `__lazy_modules__` that is not a literal**, such as a list built by a function, or one that the module changes
  after assigning it.

  ```python ok
  __lazy_modules__ = sorted(["pathlib", "json"])
  ```

### Fix

The fix is safe: CPython checks membership in the list, so its order changes nothing. A comment at the end of an entry's
line moves with the entry. Any other comment inside the list withholds the fix, since the rewrite would lose it.

```python report
__lazy_modules__ = [
    "pathlib",  # used by the CLI only
    "json",
]

import json
import pathlib
```

```python fixed
__lazy_modules__ = [
    "json",
    "pathlib",  # used by the CLI only
]

import json
import pathlib
```

## Other tools

- **[flake8-lazy](https://github.com/henryiii/flake8-lazy):** `LZY201`, without a fix. Its `--apply` mode rewrites the
  whole list instead.
- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
  [`unsorted-dunder-all`](https://docs.astral.sh/ruff/rules/unsorted-dunder-all/) (`RUF022`) sorts `__all__` only.
