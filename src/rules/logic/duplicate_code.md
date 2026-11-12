Flags six or more statements in a row that another module repeats.

## Why is this bad?

Copied code drifts. A fix lands in one copy and not the other, and nothing says that the two were ever the same. Each
change means finding and testing every copy, and readers skim past the line that differs in near-identical blocks, as
[pylint's duplicate-code page](https://pylint.readthedocs.io/en/latest/user_guide/messages/refactor/duplicate-code.html)
notes. One definition that both modules call keeps a single place to change.

Repeated text is not necessarily repeated knowledge.
[Dave Thomas](https://www.artima.com/articles/orthogonality-and-the-dry-principle) defines DRY as one authoritative
representation of each piece of knowledge, and [Sandi Metz](https://sandimetz.com/blog/2016/1/20/the-wrong-abstraction)
shows a shared function gaining a flag per caller as its callers diverge. simplint still reports textual matches, since
it cannot tell shared knowledge from coincidence. Share the code when both copies must change together.

In the example below, `reader.py` and `writer.py` each define the same six statements.

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
  in `[tool.simplint]`: the fewest statements in a row another module must repeat, 6 by default, such as
  `min-duplicate-statements = 4` under `[tool.simplint.logic]`

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

  Six is where the copies on CPython's standard library turn from boilerplate to code worth sharing: at four and five
  statements most reports are the generated codec classes of `encodings`, and at eight the copied file classes of `bz2`
  and `lzma` go unreported.

## Ruff overlap

Ruff has no rule for repeated code, so the two do not overlap.

## What it does

simplint compares the statements of every body of every module of the run, and reports a run of
`min-duplicate-statements` statements or more that another module holds too. Overlapping copies make one block, reported
once in each module on its first statement, and a block inside another one, as the body of a function the outer block
holds, is left to the outer one. The report names the other modules where their copy starts: up to three, then a count
of the rest.

Statements match when their tokens match, and the blocks they nest in match too, so the same statements under another
`if` do not. Comments, layout, indentation width, imports and docstrings do not count. Identifiers and literals must
match exactly, so a copy with a renamed variable does not match. Statements are compared by a hash of their tokens first
and by the tokens themselves before a report, so two different statements whose hashes collide never match.

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

- **Several modules repeat the block**: the report names the first three other modules in sorted order, one place each,
  and counts the rest. A module that holds the block several times still names each other module once, so its copies do
  not use up the places.

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

- **Test modules**, which the run tells by their names and the `tests` and `test` directories of the project. Tests
  repeat their setup and checks so each test reads on its own, and copy the code they test to compare against it. Stub
  files are not compared either, since stubs repeat each other's signatures by design.

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

- **Nested test projects**, which are directories named `test*`, or inside one, that hold a project of their own with a
  `conf.py`, `pyproject.toml`, `setup.py` or `setup.cfg`, such as each `roots/test-*` project of Sphinx's test suite. A
  nested project repeats code by design, so its modules are compared with each other only, never with another nested
  project or the rest of the run. Repeats within one nested project are still reported.

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

- **Runs shorter than `min-duplicate-statements`**, which are too short to report.

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

simplint offers no fix: the right shared definition depends on the code around the copies.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#17724](https://github.com/astral-sh/ruff/issues/17724).
- [pylint](https://github.com/pylint-dev/pylint) reports similar lines repeated across modules as
  [duplicate-code](https://pylint.readthedocs.io/en/latest/user_guide/messages/refactor/duplicate-code.html), with no
  fix.
