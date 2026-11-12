Flags consecutive `assert` statements in a test that each compare one field of the same object with `==`, which one
assert over a tuple would compare together.

## Why is this bad?

Each assert stops the test at its first mismatch, so a failing run shows one field and hides the others. Comparing the
fields as one tuple checks them in the same order, and pytest's
[assertion rewriting](https://docs.pytest.org/en/stable/how-to/assert.html#making-use-of-context-sensitive-comparisons)
then prints every value on both sides and the first index that differs. The tuple also drops the repeated object name.
Ruff's [PT018](https://docs.astral.sh/ruff/rules/pytest-composite-assertion/) asks to split `assert a and b`, since a
combined assertion does not name the failing condition; simplint still reports field asserts, because the tuple shows
every value in one failure.

## Example

```python report file=tests/test_user.py
def test_user(result):
    assert result.name == "x"
    assert result.size == 3
```

Use instead:

```python ok
def test_user(result):
    assert (result.name, result.size) == ("x", 3)
```

## What it does

### Active on

- **Two or more consecutive asserts on fields of one object**: each assert has no message, compares with `==` an
  attribute or a key of an object, and the object is written the same way in each. A test function is a `test*` function
  in a test module, as pytest collects it.

  ```python report file=tests/test_fields.py
  def test_attributes(result):
      assert result.name == "x"
      assert result.size == 3


  def test_keys(data):
      assert data["a"] == 1
      assert data["b"] == 2
  ```

  ```python fixed file=tests/test_fields.py
  def test_attributes(result):
      assert (result.name, result.size) == ("x", 3)


  def test_keys(data):
      assert (data["a"], data["b"]) == (1, 2)
  ```

  ```python report file=tests/test_three_fields.py
  def test_three(result):
      assert result.name == "x"
      assert result.size == 3
      assert result.color == "red"
  ```

### Exceptions

- **Asserts with a message**: the message says which check failed, which a tuple cannot.

  ```python ok file=tests/test_message.py
  def test_message(result):
      assert result.name == "x", "name"
      assert result.size == 3
  ```

- **Comparisons other than `==`**: `!=`, `<`, `in` and the rest do not compare a value to an expected one.

  ```python ok file=tests/test_comparison.py
  def test_other_comparison(result):
      assert result.name != "x"
      assert result.size > 3
  ```

- **A single assert**: there is nothing to join.

  ```python ok file=tests/test_single.py
  def test_single(result):
      assert result.name == "x"
  ```

- **Asserts separated by another statement**: the statement in between may change the object, so the asserts are not one
  check.

  ```python ok file=tests/test_separated.py
  def test_separated(result):
      assert result.name == "x"
      print(result)
      assert result.size == 3
  ```

- **Asserts on different objects**: `result` and `other` are not one object.

  ```python ok file=tests/test_objects.py
  def test_other_objects(result, other):
      assert result.name == "x"
      assert other.size == 3
  ```

### Fix

**Unsafe fix:** the failure message changes, since the tuple reports both sides of the whole comparison instead of the
first failing field. The outcome can change too:

- A tuple `==` compares items by identity before equality, so a NaN field that fails `x == x` in its own assert passes
  inside the tuple.
- The merged assert evaluates every operand, where separate asserts stop at the first failure, so a field that raises
  stops the test even where an earlier assert would have failed.

The fix replaces the run with one assert that keeps the original order of the fields, as in the example above.

No fix is offered when:

- the joined assert would be wider than the line length, measured as the formatter measures a joined line;

  ```python report file=tests/test_long_line.py
  def test_long_line(result):
      assert result.some_really_long_attribute_name_here == "a value that is long enough to exceed"
      assert result.another_really_long_attribute_name == "another value that makes it too long"
  ```

- a comment sits on one of the asserts, which the replacement would drop.

  ```python report file=tests/test_comment.py
  def test_comment(result):
      assert result.name == "x"  # the name
      assert result.size == 3
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** released rule
  [PT018](https://docs.astral.sh/ruff/rules/pytest-composite-assertion/) splits one assertion joined by `and` or `or`;
  this rule combines consecutive field comparisons into one tuple comparison. Neither reports the other's cases.
