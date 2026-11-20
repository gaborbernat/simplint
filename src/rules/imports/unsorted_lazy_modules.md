Flags a [`__lazy_modules__`](https://docs.python.org/3.15/reference/simple_stmts.html#compatibility-via-lazy-modules)
list whose entries are not sorted.

## Why is this bad?

A predictable order makes it easier to find a module in a long list and to spot duplicate entries.

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

- **A computed declaration or another use of `__lazy_modules__` in code**. The rule needs a single assignment whose
  entries it can read.

  ```python ok
  __lazy_modules__ = sorted(["pathlib", "json"])
  ```

### Fix

The fix is safe because Python checks membership in `__lazy_modules__`. It keeps each entry's trailing comment with that
entry. Other comments inside the list or a tuple written without parentheses prevent the fix.

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

- [flake8-lazy](https://github.com/henryiii/flake8-lazy) reports unsorted entries as `LZY201`. Its standalone
  `--apply=list` mode regenerates the list.
- Ruff's [`unsorted-dunder-all`](https://docs.astral.sh/ruff/rules/unsorted-dunder-all/) (`RUF022`) sorts `__all__`.
