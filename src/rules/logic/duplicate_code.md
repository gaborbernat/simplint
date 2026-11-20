Flags consecutive statements repeated in another module. The default minimum is six statements.

## Why is this bad?

When two modules implement the same behavior, fixing one copy can leave the other with the original bug. A shared
definition lets both callers pick up the fix.

Share copies that need to change together.
[Dave Thomas](https://www.artima.com/articles/orthogonality-and-the-dry-principle) describes DRY in terms of shared
knowledge. Identical text can also be coincidental.
[Sandi Metz](https://sandimetz.com/blog/2016/1/20/the-wrong-abstraction) shows how a shared routine accumulates
conditional branches as its callers' requirements diverge.

## Example

```python report file=reader.py
def parse_lines(text):
    lines = text.splitlines()
    values = {}
    for line in lines:
        key, _, value = line.partition("=")
        values[key] = value
    count = len(values)
    if count == 0:
        raise ValueError("no values")
    return values, count
```

```python report file=writer.py
def parse_lines(text):
    lines = text.splitlines()
    values = {}
    for line in lines:
        key, _, value = line.partition("=")
        values[key] = value
    count = len(values)
    if count == 0:
        raise ValueError("no values")
    return values, count
```

Move the parser into one module that both import:

```python context file=parsing.py
def parse_lines(text):
    lines = text.splitlines()
    values = {}
    for line in lines:
        key, _, value = line.partition("=")
        values[key] = value
    count = len(values)
    if count == 0:
        raise ValueError("no values")
    return values, count
```

```python ok file=reader.py
from parsing import parse_lines
```

## Configuration

- [`logic.min-duplicate-statements`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#logic)
  sets the minimum length of a matching block. The default is six. Setting it to zero has the same effect as setting it
  to one.

  ```toml config
  [tool.simplint.logic]
  min-duplicate-statements = 4
  ```

  ```python context file=first.py
  def run(items):
      total = sum(items)
      count = len(items)
      log(total)
      return total, count
  ```

  ```python report file=second.py
  def run(items):
      total = sum(items)
      count = len(items)
      log(total)
      return total, count
  ```

## What it does

simplint compares runs of statements from a single body, such as a function or an `if` branch, against runs in other
modules. It merges overlapping matches and reports the outermost block when one match contains another. Each diagnostic
lists up to three other files and the starting line in each. If more files share the code, it gives their count.

The tokens must match exactly, including identifiers, literal spellings and indentation changes *within* a statement.
The surrounding function or branch can differ. Comments, spacing and indentation width do not affect the comparison. The
rule skips import statements and standalone string literals when counting consecutive statements. Their tokens still
matter when comparing a compound statement that contains them. It checks matching hashes against the token text before
reporting a duplicate.

### Active on

- **Six consecutive statements of one body** that another module repeats, at any depth, such as the body of a function,
  a method or an `if` block. A nested block, such as a `for` loop, counts as one statement.

  ```python context file=first.py
  def run(items):
      total = 0
      for item in items:
          total += item
      count = len(items)
      log(total)
      log(count)
      return total
  ```

  ```python report file=second.py
  def run(items):
      total = 0
      for item in items:
          total += item
      count = len(items)
      log(total)
      log(count)
      return total
  ```

- **Several modules repeat the block.** Each file appears at most once in the diagnostic, even if it contains several
  copies.

  ```python report file=alpha.py
  def run(items):
      total = sum(items)
      count = len(items)
      log(total)
      log(count)
      checked = total > 0
      return checked


  def run_again(items):
      total = sum(items)
      count = len(items)
      log(total)
      log(count)
      checked = total > 0
      return checked
  ```

  ```python context file=beta.py
  def run(items):
      total = sum(items)
      count = len(items)
      log(total)
      log(count)
      checked = total > 0
      return checked
  ```

  ```python context file=delta.py
  def run(items):
      total = sum(items)
      count = len(items)
      log(total)
      log(count)
      checked = total > 0
      return checked
  ```

  ```python context file=epsilon.py
  def run(items):
      total = sum(items)
      count = len(items)
      log(total)
      log(count)
      checked = total > 0
      return checked
  ```

  ```python context file=gamma.py
  def run(items):
      total = sum(items)
      count = len(items)
      log(total)
      log(count)
      checked = total > 0
      return checked
  ```

### Exceptions

- **Separate distributions in one repository**: files are compared within the distribution identified by their nearest
  project metadata. For example, code repeated in two workspace members with separate `[project]` tables is not reported
  across those members. Repeats within either member are still reported.

  ```toml config
  [tool.uv.workspace]
  members = ["first", "second"]
  ```

  ```toml context file=first/pyproject.toml
  [project]
  name = "first"
  version = "1"
  ```

  ```toml context file=second/pyproject.toml
  [project]
  name = "second"
  version = "1"
  ```

  ```python ok file=first/reader.py
  def summarize(items):
      total = 0
      for item in items:
          total += item
      count = len(items)
      log(total)
      log(count)
      return total
  ```

  ```python ok file=second/writer.py
  def summarize(items):
      total = 0
      for item in items:
          total += item
      count = len(items)
      log(total)
      log(count)
      return total
  ```

- **Repeats inside one module**, which pylint's duplicate check does not compare either.

  ```python ok file=twice.py
  def first(items):
      total = 0
      for item in items:
          total += item
      count = len(items)
      log(total)
      log(count)
      return total


  def second(items):
      total = 0
      for item in items:
          total += item
      count = len(items)
      log(total)
      log(count)
      return total
  ```

- **Tests and stubs.** simplint skips modules named `test_*.py`, `*_test.py` or `conftest.py`, files matched by pytest's
  `python_files` within any configured `testpaths` directories, and files under `test/` or `tests/`. It also skips
  `.pyi` files, where unrelated APIs can share signatures.

  ```python context file=tests/test_first.py
  def test_run():
      total = 0
      for item in ITEMS:
          total += item
      count = len(ITEMS)
      log(total)
      log(count)
      assert total
  ```

  ```python ok file=tests/test_second.py
  def test_run():
      total = 0
      for item in ITEMS:
          total += item
      count = len(ITEMS)
      log(total)
      log(count)
      assert total
  ```

- **Nested test projects.** A directory at or below `test`, `tests`, `test-*` or `test_*` starts a separate project if
  it contains `conf.py`, `pyproject.toml`, `setup.py` or `setup.cfg`. For example, modules in `roots/test-first` can
  match each other but are compared separately from `roots/test-second`. The usual exclusion for test modules still
  applies.

  ```python context file=roots/test-first/conf.py
  project = "first"
  ```

  ```python context file=roots/test-first/parser.py
  def run(items):
      total = 0
      for item in items:
          total += item
      count = len(items)
      log(total)
      log(count)
      return total
  ```

  ```python context file=roots/test-second/conf.py
  project = "second"
  ```

  ```python ok file=roots/test-second/parser.py
  def run(items):
      total = 0
      for item in items:
          total += item
      count = len(items)
      log(total)
      log(count)
      return total
  ```

- **Matching `_sync` and `_async` paths.** The rule skips a module under `_sync` if the run includes its counterpart
  under `_async`. [unasync](https://github.com/python-trio/unasync) uses this layout for generated modules.

  ```python context file=pkg/_async/parser.py
  async def run(items):
      total = 0
      async for item in items:
          total += item
      count = len(items)
      log(total)
      log(count)
      return total
  ```

  ```python ok file=pkg/_sync/parser.py
  def run(items):
      total = 0
      for item in items:
          total += item
      count = len(items)
      log(total)
      log(count)
      return total
  ```

- **Runs shorter than `min-duplicate-statements`.**

  ```python context file=short_first.py
  def run(items):
      total = 0
      for item in items:
          total += item
      count = len(items)
      return total, count
  ```

  ```python ok file=short_second.py
  def run(items):
      total = 0
      for item in items:
          total += item
      count = len(items)
      return total, count
  ```

### Undetected cases

Code repeated with renamed identifiers, changed literals or reordered statements does not match.

### Fix

Choose whether the copies should change together and where their shared definition belongs. The rule offers no automatic
fix.

## Other tools

- [Ruff issue #17724](https://github.com/astral-sh/ruff/issues/17724) requests code duplication detection.
- [pylint](https://github.com/pylint-dev/pylint) reports similar lines repeated across modules as
  [duplicate-code](https://pylint.readthedocs.io/en/latest/user_guide/messages/refactor/duplicate-code.html), with no
  fix.
