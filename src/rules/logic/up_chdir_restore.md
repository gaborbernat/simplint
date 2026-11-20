Flags code that saves the working directory, changes it, and restores it in a `finally` clause.

## Why is this bad?

Since Python 3.11, [`contextlib.chdir(target)`](https://docs.python.org/3/library/contextlib.html#contextlib.chdir)
restores the previous working directory when the `with` block exits, including when the body raises an exception. Both
forms change the working directory for the whole process.

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

The rule looks for three consecutive statements. The first saves
[`os.getcwd()`](https://docs.python.org/3/library/os.html#os.getcwd) under a name. The second calls
[`os.chdir(target)`](https://docs.python.org/3/library/os.html#os.chdir). The third is a `try` block with only
`os.chdir(saved)` in its `finally` clause and no exception handlers. The saved name must appear only in the assignment
and the restore within its scope.

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

simplint marks this fix as safe. If evaluating the target expression itself changes the working directory, the two forms
can restore different directories. The original saves the directory before evaluating the target. `contextlib.chdir`
saves it on entry, after the target has been evaluated.

The fix uses an existing `import contextlib` or adds one to the standard library imports. simplint reports without a fix
if the name would refer to another binding. It also skips the edit if it would remove a comment, if the assignment
follows another statement on the same line or continues the previous line, or if the `try` body starts on the `try`
line.

For a module-level assignment, the fix also checks whether another analyzed module reads the saved name through an
import, star import or attribute, or whether `__all__` exports it. Those uses still need the assignment.

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

When the fix needs an import, it checks the insertion point. An import before the module docstring would change
`__doc__`. An import before a `from __future__` import would cause a syntax error. The fix also skips an insertion after
an import that shares its line with another statement.

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

The fix keeps comments with the import they precede.

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

A parameter named `contextlib` hides the module import inside the function.

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

A star import in another analyzed module can expose the saved name.

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
