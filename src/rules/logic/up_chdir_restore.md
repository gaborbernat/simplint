Flags a directory change that a `try` block undoes in its `finally` clause, which `contextlib.chdir` does from Python
3.11.

## Why is this bad?

Saving the working directory, changing it and restoring it in a `finally` clause takes three statements and a `try`,
where `contextlib.chdir` needs one `with` block. The block also cannot forget the restore.

## Example

```toml config
[project]
requires-python = ">=3.11"
```

```python report
import os


def build(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project()
    finally:
        os.chdir(old)
```

Use instead:

```python ok
import contextlib


def build(target):
    with contextlib.chdir(target):
        return compile_project()
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python ok
  import os


  def build(target):
      old = os.getcwd()
      os.chdir(target)
      try:
          return compile_project()
      finally:
          os.chdir(old)
  ```

## What it does

Reports three statements in one body: `old = os.getcwd()`, then `os.chdir(target)` with one argument, then a `try` whose
only clause is a `finally` that holds `os.chdir(old)`, with no `except` and no `else`. The name `old` must appear
nowhere else in its function, or in the module at module level.

### Active on

```toml config
[project]
requires-python = ">=3.11"
```

```python report
import os


def run(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        step()
    finally:
        os.chdir(old)
```

```python fixed
import contextlib


def run(target):
    with contextlib.chdir(target):
        step()
```

### Exceptions

- **The saved directory read after the block**: the name has another use.

  ```toml config
  [project]
  requires-python = ">=3.11"
  ```

  ```python ok
  import os


  def run(target):
      old = os.getcwd()
      os.chdir(target)
      try:
          step()
      finally:
          os.chdir(old)
      print(old)
  ```

- **A `finally` that changes the directory to another place**: the restore does not return to the saved directory.

  ```toml config
  [project]
  requires-python = ">=3.11"
  ```

  ```python ok
  import os


  def run(target):
      old = os.getcwd()
      os.chdir(target)
      try:
          step()
      finally:
          os.chdir(target)
  ```

- **Target versions before 3.11**: `contextlib.chdir` does not exist there.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python ok
  import os


  def run(target):
      old = os.getcwd()
      os.chdir(target)
      try:
          step()
      finally:
          os.chdir(old)
  ```

### Fix

The fix is safe under those conditions. It adds `import contextlib` before the module's first import, unless the module
imports `contextlib` already. When another binding holds the name `contextlib`, in the module or by a parameter, local
or loop variable of a function, lambda, class body or comprehension that encloses the assignment, the rule reports
without a fix. It also reports without a fix when a comment sits in the lines it would remove, when the assignment
shares its line with an earlier statement, or when the line before it ends with a backslash that joins the two lines. A
module-level `old` that another module of the run reads, through an import, a star import or an attribute, or that
`__all__` lists, keeps its report without a fix, since removing the assignment would break those readers.

```toml config
[project]
requires-python = ">=3.11"
```

```python report
import os


def run(target):
    # fmt: off
    log("start"); old = os.getcwd()
    # fmt: on
    os.chdir(target)
    try:
        step()
    finally:
        os.chdir(old)
```

```python fixed
import os


def run(target):
    # fmt: off
    log("start"); old = os.getcwd()
    # fmt: on
    os.chdir(target)
    try:
        step()
    finally:
        os.chdir(old)
```

A module docstring that shares its line with code keeps the report without a fix, since an import above it would leave
`__doc__` as `None`. A `from __future__` import that shares its line with code does the same, since an import above it
is a syntax error. So does a last import that shares its line with code, since the new import would land below that
code.

```toml config
[project]
requires-python = ">=3.11"
```

```python report
# fmt: off
import abc; NAME = abc.__name__
# fmt: on
import os


def run(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return os.listdir()
    finally:
        os.chdir(old)
```

```python fixed
# fmt: off
import abc; NAME = abc.__name__
# fmt: on
import os


def run(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return os.listdir()
    finally:
        os.chdir(old)
```

```toml config
[project]
requires-python = ">=3.11"
```

```python report
# fmt: off
"""Build helpers."""; import os
# fmt: on


def build(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project()
    finally:
        os.chdir(old)
```

```python fixed
# fmt: off
"""Build helpers."""; import os
# fmt: on


def build(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project()
    finally:
        os.chdir(old)
```

```toml config
[project]
requires-python = ">=3.11"
```

```python report
import os


def run(target):
    # fmt: off
    started = True; \
    old = os.getcwd()
    # fmt: on
    os.chdir(target)
    try:
        step()
    finally:
        os.chdir(old)
```

```python fixed
import os


def run(target):
    # fmt: off
    started = True; \
    old = os.getcwd()
    # fmt: on
    os.chdir(target)
    try:
        step()
    finally:
        os.chdir(old)
```

```toml config
[project]
requires-python = ">=3.11"
```

```python context file=user.py
from state import old
```

```python report file=state.py
import contextlib
import os

old = os.getcwd()
os.chdir("..")
try:
    print(os.getcwd())
finally:
    os.chdir(old)
```

```python fixed file=state.py
import contextlib
import os

old = os.getcwd()
os.chdir("..")
try:
    print(os.getcwd())
finally:
    os.chdir(old)
```

The import goes above the comment lines that sit directly over the import it precedes, so the comment stays with its
statement:

```toml config
[project]
requires-python = ">=3.11"
```

```python report
import abc

# os: the build runs in the target directory
import os


def run(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return abc, os.listdir()
    finally:
        os.chdir(old)
```

```python fixed
import abc
import contextlib

# os: the build runs in the target directory
import os


def run(target):
    with contextlib.chdir(target):
        return abc, os.listdir()
```

A parameter named `contextlib` keeps the report without a fix, even where the module imports `contextlib`:

```toml config
[project]
requires-python = ">=3.11"
```

```python report
import contextlib
import os


def run(target, contextlib=None):
    old = os.getcwd()
    os.chdir(target)
    try:
        return os.listdir()
    finally:
        os.chdir(old)
```

```python fixed
import contextlib
import os


def run(target, contextlib=None):
    old = os.getcwd()
    os.chdir(target)
    try:
        return os.listdir()
    finally:
        os.chdir(old)
```

A star import of the module counts as a read of the name, so the report stays without a fix:

```toml config
[project]
requires-python = ">=3.11"
```

```python context file=user.py
from state import *

print(old)
```

```python report file=state.py
import contextlib
import os

old = os.getcwd()
os.chdir("..")
try:
    print(os.getcwd())
finally:
    os.chdir(old)
```

```python fixed file=state.py
import contextlib
import os

old = os.getcwd()
os.chdir("..")
try:
    print(os.getcwd())
finally:
    os.chdir(old)
```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
