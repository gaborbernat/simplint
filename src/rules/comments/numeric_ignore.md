Flags numeric codes in `# simplint: ignore[...]` comments and replaces them with rule names.

## Why is this bad?

The name says what the suppression covers without looking up a code. It also stays readable when the comment is reviewed
outside a diagnostic report.

## Example

```python report
def initialize(build_data: dict[str, Any]) -> None:  # simplint: ignore[SMP602] # required signature
    pass
```

```python fixed
def initialize(build_data: dict[str, Any]) -> None:  # simplint: ignore[imprecise-annotation] # required signature
    pass
```

## What it does

### Active on

- **Known numeric simplint codes** in `simplint: ignore[...]`. The fix replaces only the codes and preserves the other
  names, spacing, and reason in the comment.

  ```python report
  def run(value: Any) -> None:  # simplint: ignore[SMP602, single-letter-name] # external signature
      pass
  ```

### Exceptions

- **Rule names** already use the preferred form. Unknown codes are left as written.

  ```python ok
  def run(value: Any) -> None:  # simplint: ignore[imprecise-annotation] # external signature
      pass
  ```

### Fix

The fix is safe and changes only known numeric codes inside the brackets.
