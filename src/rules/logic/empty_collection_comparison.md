Flags `==` and `!=` comparisons with an empty list, tuple or dict literal. Use a truth test when any empty collection
should match.

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

- **A single `==` or `!=` comparison with `[]`, `()` or `{}` on exactly one side.** The unsafe fix replaces `x == []`
  with `not x` and `x != []` with `x` when only the truth value matters. This includes conditions, comprehension
  filters, `case` guards, `not`, `bool()` and `and` or `or` within a truth test. `if not x == []:` becomes `if x:`,
  while `return not x == []` becomes `return bool(x)`. Check the expected type before applying the fix. For example,
  `None` is falsy but does not equal `[]`.

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

- **`assert` statements.** An assertion may need to distinguish an empty list from an empty tuple.

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

- **Lookups that default to `None`.** The rule skips `.get(key)`, `.get(key, None)` and
  [`getattr(obj, name, None)`](https://docs.python.org/3/builtins/functions.html#getattr) so a missing key or attribute
  stays distinguishable from an empty collection.

- **Empty strings**: Ruff's [compare-to-empty-string](https://docs.astral.sh/ruff/rules/compare-to-empty-string/)
  (PLC1901) reports `== ""`, so this rule leaves it to Ruff.

  ```python ok
  def is_blank(text):
      return text == ""
  ```

## Other tools

- [Pylint](https://github.com/pylint-dev/pylint) also checks comparisons with empty collections in
  [use-implicit-booleaness-not-comparison](https://pylint.readthedocs.io/en/stable/user_guide/messages/convention/use-implicit-booleaness-not-comparison.html)
  (C1803), with no fix.
