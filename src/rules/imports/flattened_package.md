Flags private modules whose names share a `_<prefix>_` prefix with two or more sibling modules, which a package would
group.

## Why is this bad?

Modules such as `_html_url.py`, `_html_parse.py` and `_html_css.py` spread one private implementation over files in a
flat directory, and only their names say that they belong together. A `_html/` package holds the same modules under
their own names, and its `__init__.py` re-exports the names callers use, so the package keeps its internals private
while callers import from `_html` alone.

## Example

```python report file=pkg/_html_url.py
def parse_url(text: str) -> str:
    return text.strip()
```

```python context file=pkg/_html_parse.py
def parse_tag(text: str) -> str:
    return text.strip()
```

```python context file=pkg/_html_css.py
def parse_style(text: str) -> str:
    return text.strip()
```

Use instead:

```python context file=pkg/_html/_url.py
def parse_url(text: str) -> str:
    return text.strip()
```

```python context file=pkg/_html/_parse.py
def parse_tag(text: str) -> str:
    return text.strip()
```

```python context file=pkg/_html/_css.py
def parse_style(text: str) -> str:
    return text.strip()
```

```python ok file=pkg/_html/__init__.py
from ._css import parse_style
from ._parse import parse_tag
from ._url import parse_url

__all__ = ["parse_style", "parse_tag", "parse_url"]
```

```python ok file=pkg/b.py
from ._html import parse_url

parse_url("text")
```

## What it does

The rule reports a `_<prefix>_<rest>.py` module once, at the start of the file, when at least three private modules of
its directory share the `_<prefix>_` prefix, itself included. A stub counts toward its prefix, once when it sits beside
its `.py` module. The message names the number of other modules.

### Active on

- **Modules sharing a prefix**: `_csv_read.py` with `_csv_parse.py` and `_csv_write.py` in the same directory.

  ```python context file=pkg/_csv_parse.py
  VALUE = 1
  ```

  ```python context file=pkg/_csv_write.py
  VALUE = 1
  ```

  ```python report file=pkg/_csv_read.py
  VALUE = 1
  ```

- **A `_<prefix>.py` module**: `_csv.py` beside the group makes a new package conflict with it, so the message asks to
  move it into the package as its `__init__.py`.

  ```python context file=pkg/_csv.py
  VALUE = 1
  ```

  ```python context file=pkg/_csv_parse.py
  VALUE = 1
  ```

  ```python context file=pkg/_csv_write.py
  VALUE = 1
  ```

  ```python report file=pkg/_csv_read.py
  VALUE = 1
  ```

- **A stub beside the group**: `_csv_write.pyi` counts toward the prefix without a `.py` module of its own.

  ```python context file=pkg/_csv_parse.py
  VALUE = 1
  ```

  ```python context file=pkg/_csv_write.pyi
  VALUE: int
  ```

  ```python report file=pkg/_csv_read.py
  VALUE = 1
  ```

### Exceptions

- **Fewer than three modules**: two modules sharing a prefix are left alone, such as `_csv_read.py` with
  `_csv_parse.py`.

  ```python context file=pkg/_csv_parse.py
  VALUE = 1
  ```

  ```python ok file=pkg/_csv_read.py
  VALUE = 1
  ```

- **A package for the prefix**: a `_csv/` package that holds files of the run already groups the modules, so the flat
  modules beside it are left alone, whether or not the run passes its files.

  ```python context file=pkg/_csv/__init__.py
  ```

  ```python context file=pkg/_csv_parse.py
  VALUE = 1
  ```

  ```python context file=pkg/_csv_write.py
  VALUE = 1
  ```

  ```python ok file=pkg/_csv_read.py
  VALUE = 1
  ```

- **Public modules**: `html_url.py` has no leading underscore, so it is public and never counts.

  ```python context file=pkg/html_parse.py
  VALUE = 1
  ```

  ```python context file=pkg/html_css.py
  VALUE = 1
  ```

  ```python ok file=pkg/html_url.py
  VALUE = 1
  ```

- **Dunder files**: `__init__.py` and other dunder names never count as private modules, even beside modules that share
  a prefix.

  ```python context file=pkg/_html_url.py
  VALUE = 1
  ```

  ```python context file=pkg/_html_parse.py
  VALUE = 1
  ```

  ```python context file=pkg/_html_css.py
  VALUE = 1
  ```

  ```python ok file=pkg/__init__.py
  VALUE = 1
  ```

- **A directory without `__init__.py`**: a `_csv/` folder that holds only data files is not a package, so the modules
  beside it are still reported.

  ```rst context file=pkg/_csv/notes.rst
  Notes.
  ```

  ```python report file=pkg/_csv_read.py
  VALUE = 1
  ```

  ```python context file=pkg/_csv_parse.py
  VALUE = 1
  ```

  ```python context file=pkg/_csv_write.py
  VALUE = 1
  ```

## Fix

No fix is offered. Moving the modules into a package changes their import paths, and the package's `__init__.py` needs
the names callers use.

## Other tools

[Ruff](https://github.com/astral-sh/ruff) has no rule for private modules that share a name prefix in place of a
package.
