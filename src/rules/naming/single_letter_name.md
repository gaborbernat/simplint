Flags single-letter names outside comprehensions.

## Why is this bad?

A single letter says nothing about what the name holds, so the reader has to find where it is bound to learn it.
Comprehensions are exempt because the iterable the names come from sits right next to them.

## Example

```python report
def scale(v: float, f: float) -> float:
    r = v * f
    return round(r, 2)
```

Use instead:

```python ok
def scale(value: float, factor: float) -> float:
    scaled = value * factor
    return round(scaled, 2)
```

## What it does

No fix is offered, since a meaningful name cannot be derived from the code.

### Active on

- **Functions and classes**: their names show up at every call site, far from the definition that explains them.

  ```python report
  def f() -> int:
      return 1
  ```

- **Parameters**: the body reads them away from the signature, and callers see the name in the signature and keyword
  arguments. This covers:

  - Parameters of a `def`, such as `v` in `def scale(v: float)`:

    ```python report
    def scale(v: float) -> float:
        return v * 2
    ```

  - Parameters of a [lambda](https://docs.python.org/3/reference/expressions.html#lambda), such as `v` in `lambda v: v`:

    ```python report
    callback = lambda v: v
    ```

- **Assignment and [walrus](https://docs.python.org/3/reference/expressions.html#assignment-expressions) targets**: the
  value is computed once, and every later use relies on the name alone to say what it is.

  - `=` assignments, such as `r = 3`:

    ```python report
    r = 3
    print(r)
    ```

  - `:=` walrus targets, such as `n` in `if (n := 3) > 2`:

    ```python report
    if (n := 3) > 2:
        print(n)
    ```

- **`with ... as`, `except ... as` and `import ... as` names**: the alias replaces the expression, exception or module
  name that said what the value is.

  - `with ... as` names, such as `f` in `with open("data.txt") as f`:

    ```python report
    with open("data.txt") as f:
        print(f.read())
    ```

  - `except ... as` names, such as `e` in `except ValueError as e`:

    ```python report
    try:
        run()
    except ValueError as e:
        print(e)
    ```

  - `import ... as` names, such as `o` in `import os as o`:

    ```python report
    import os as o
    ```

- **[`match`](https://docs.python.org/3/reference/compound_stmts.html#the-match-statement) captures**: the case body
  reads the captured value under that name.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python report
  value = 1
  match value:
      case x:
          print(x)
  ```

- **Mapping rest captures**: `**rest` in a mapping pattern binds the remaining keys, and the case body reads them under
  that name.

  ```python report
  value = {"a": 1, "b": 2}
  match value:
      case {"a": 1, **r}:
          print(r)
  ```

- **[Type parameters](https://docs.python.org/3/reference/compound_stmts.html#type-params)** such as `T`: they appear
  throughout the signature and body of the generic definition.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python report
  def first[T](items: list[T]) -> T:
      return items[0]
  ```

- **`for` loop targets**, such as `i` in `for i in range(3)`: a loop body can run for many lines, far from the `for`
  line that says what the name holds.

  ```python report
  for i in range(3):
      print(i)
  ```

### Exceptions

- **Names inside a comprehension**: every name inside a list, set or dictionary
  [comprehension](https://docs.python.org/3/tutorial/datastructures.html#list-comprehensions) or a
  [generator expression](https://docs.python.org/3/reference/expressions.html#generator-expressions), including a walrus
  target or lambda parameter, so `[x * 2 for x in values]` is fine. The whole expression keeps the iterable next to
  every use of the name.

  ```python ok
  values = [1, 2]
  doubled = [x * 2 for x in values]
  ```

- **`_`**: the conventional throwaway name, which says the value is unused.

  ```python ok
  for _ in range(3):
      pass
  ```

- **Parameters of a method decorated with
  [`@override`](https://docs.python.org/3/library/typing.html#typing.override)**: the method must keep the parameter
  names of the method it replaces.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python ok
  from typing import override


  class Base:
      def run(self, value: int) -> None:
          pass


  class Child(Base):
      @override
      def run(self, v: int) -> None:
          pass
  ```

### Reporting

Each name is reported once per scope, at its first binding.

Ruff's [ambiguous-variable-name](https://docs.astral.sh/ruff/rules/ambiguous-variable-name/) covers only `l`, `O` and
`I`.

## Other tools

- [flake8-variables-names](https://github.com/best-doctor/flake8-variables-names) reports single-letter names as
  [VNE001](https://github.com/best-doctor/flake8-variables-names/blob/master/flake8_variables_names/checker.py), but
  allows `i`, `_` and `T` by default. This rule flags `for i` loop targets and type parameters such as `T`.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports short names as
  [WPS111](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/naming.html#wemake_python_styleguide.violations.naming.TooShortNameViolation),
  with a configurable minimum length. It also checks module and attribute names, which this rule does not list.
