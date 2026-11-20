Flags `==` and `!=` comparisons between a prefix or suffix slice and a nonempty literal of the same length.
[`str.startswith()`](https://docs.python.org/3/builtins/stdtypes.html#str.startswith) and
[`str.endswith()`](https://docs.python.org/3/builtins/stdtypes.html#str.endswith) test the same without a length to keep
in step.

## Why is this bad?

[PEP 8](https://peps.python.org/pep-0008/#programming-recommendations) asks for `startswith()` and `endswith()` instead
of slicing. The slice repeats the literal's length as a number. If you change `"://"` to `":/"` but leave `[:3]` in
place, the slice can include a character past the prefix. `startswith()` measures the prefix itself.

## Example

```python report
def is_comment(line):
    return line[:2] == "//"
```

Use instead:

```python ok
def is_comment(line):
    return line.startswith("//")
```

## What it does

### Active on

- **`==` and `!=` between a `[:n]` or `[-n:]` slice and a string or bytes literal of length `n`**, on either side.
  **Unsafe fix:** when the sliced value is surely a `str` or `bytes`, such as a parameter annotated so, the comparison
  becomes a `startswith()` or `endswith()` call, negated with `not` for `!=`. It is unsafe because a subclass may
  override the method or the slice. Any other value is reported without a fix: a class may compare its slices equal to a
  string, as pyasn1's character types do, without having `startswith`.

  ```python report
  def kind(name: str):
      return name[-3:] == ".py", name[:2] != "__"
  ```

  ```python fixed
  def kind(name: str):
      return name.endswith(".py"), not name.startswith("__")
  ```

### Exceptions

- **A length that differs from the literal's length**, or an empty literal. Changing the comparison to `startswith()` or
  `endswith()` could change its result.

  ```python ok
  def is_comment(line):
      return line[:3] == "//"
  ```

- **Slices with a start and a stop, or a step**, and comparisons with anything but a literal.

  ```python ok
  def is_tag(line, tag):
      return line[1:3] == "ab" or line[:3] == tag
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** none;
  [slice-to-remove-prefix-or-suffix](https://docs.astral.sh/ruff/rules/slice-to-remove-prefix-or-suffix/) (FURB188)
  covers the slice that removes a prefix after `startswith()`.
