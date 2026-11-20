Flags numeric codes in `# simplint: ignore[...]` comments and replaces them with rule names.

## Why is this bad?

A reviewer can read `imprecise-annotation` directly in a diff. `SMP602` requires a lookup. Pylint's
[use-symbolic-message-instead](https://pylint.readthedocs.io/en/latest/user_guide/messages/information/use-symbolic-message-instead.html)
makes the same distinction for its suppressions.

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
