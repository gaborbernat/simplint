Flags `s[:n] == literal` and `s[-n:] == literal` when the literal is `n` characters or bytes long.
[`str.startswith()`](https://docs.python.org/3/library/stdtypes.html#str.startswith) and
[`str.endswith()`](https://docs.python.org/3/library/stdtypes.html#str.endswith) test the same without a length to keep
in step.

## Why is this bad?

[PEP 8](https://peps.python.org/pep-0008/#programming-recommendations) asks for `startswith()` and `endswith()` instead
of slicing. The slice repeats the literal's length as a number, and when the literal changes and the number does not,
the comparison is always false, with no error.

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
  **Unsafe fix:** the comparison becomes a `startswith()` or `endswith()` call, negated with `not` for `!=`. It is
  unsafe because the receiver's type is not known: a list slice equals no string, while a list has no `startswith`.

  ```python report
  def kind(name):
      return name[-3:] == ".py", name[:2] != "__"
  ```

  ```python fixed
  def kind(name):
      return name.endswith(".py"), not name.startswith("__")
  ```

### Exceptions

- **A length that does not match the literal**: the comparison is always false, a different mistake.

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
