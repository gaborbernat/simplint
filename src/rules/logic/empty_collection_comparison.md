Flags `==` and `!=` comparisons with an empty list, tuple or dict literal. A truth test says the same without building a
new object to compare with.

## Why is this bad?

[PEP 8](https://peps.python.org/pep-0008/#programming-recommendations) asks to "use the fact that empty sequences are
false". `items == []` also builds a list on each run, and it is `False` for an empty tuple or set, so it rejects empty
values the code meant to accept once a caller passes another collection type.

## Example

```python report
def first(items):
    if items == []:
        return None
    return items[0]
```

Use instead:

```python ok
def first(items):
    if not items:
        return None
    return items[0]
```

## What it does

### Active on

- **`==` and `!=` with `[]`, `()` or `{}` on either side.** **Unsafe fix:** when only the comparison's truth is used, as
  the test of an `if`, `elif`, `while`, conditional expression, comprehension filter or `case` guard, or under `not`,
  `bool()`, `and` or `or`, `x == []` becomes `not x` and `x != []` becomes `x`; `not x == []` becomes `x`. It is unsafe
  because `None`, `0` and `""` are falsy too, while only an empty list equals `[]`.

  ```python report
  def first(items):
      if items == []:
          return None
      return items[0]
  ```

  ```python fixed
  def first(items):
      if not items:
          return None
      return items[0]
  ```

- **Comparisons elsewhere**, such as in a `return` or an argument, reported without a fix: replacing a `bool` with the
  value itself changes the result.

  ```python report
  def is_empty(items):
      return items == []
  ```

### Exceptions

- **`assert` statements**: a test that a result equals `[]` also checks that it is a list.

  ```python ok
  def test_empty(result):
      assert result == []
  ```

- **Comparisons next to one with `None` or another literal**, as in `value == [] or value is None`, which tell the falsy
  values apart on purpose.

  ```python ok
  def is_blank(value):
      return value == [] or value is None
  ```

- **Empty strings**: Ruff's [compare-to-empty-string](https://docs.astral.sh/ruff/rules/compare-to-empty-string/)
  (PLC1901) reports `== ""`, so this rule leaves it to Ruff.

  ```python ok
  def is_blank(text):
      return text == ""
  ```

## Other tools

- [Pylint](https://github.com/pylint-dev/pylint) reports the same comparisons as
  [use-implicit-booleaness-not-comparison](https://pylint.readthedocs.io/en/stable/user_guide/messages/convention/use-implicit-booleaness-not-comparison.html)
  (C1803), with no fix.
