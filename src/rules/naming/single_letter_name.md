Flags single-letter names outside comprehensions and conventional type parameter declarations.

## Why is this bad?

A single letter says nothing about what the name holds, so the reader has to find where it is bound to learn it.
[PEP 8](https://peps.python.org/pep-0008/#names-to-avoid) bans only `l`, `O` and `I`, which some fonts confuse with
digits. The [Google Python Style Guide](https://google.github.io/styleguide/pyguide.html#s3.16.1-names-to-avoid) allows
`i` for loop counters and `e` and `f` in `except` and `with`; simplint reports those forms too, since the letter alone
still says nothing about the value. Comprehensions are exempt because the iterable the names come from sits right next
to them.

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

- **Parameters of a `def`**, such as `v` in `def scale(v: float)`: the body reads them away from the signature, and
  callers see the name in the signature and keyword arguments.

  ```python report
  def scale(v: float) -> float:
      return v * 2
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

- **Parameters of a [lambda](https://docs.python.org/3/reference/expressions.html#lambda)**: a lambda is a single
  expression, so like a comprehension it keeps each parameter next to every use.

  ```python ok
  def order(values: list[int]) -> list[int]:
      return sorted(values, key=lambda v: (v < 0, abs(v)))
  ```

- **`_`**: the conventional throwaway name, which says the value is unused.

  ```python ok
  for _ in range(3):
      pass
  ```

- **Uppercase type parameters**: names such as `T` and `P` are conventional in generic signatures. This includes
  parameters in [PEP 695 syntax](https://docs.python.org/3/reference/compound_stmts.html#type-params) and assignments to
  [`typing.TypeVar`](https://docs.python.org/3/library/typing.html#typing.TypeVar),
  [`typing.TypeVarTuple`](https://docs.python.org/3/library/typing.html#typing.TypeVarTuple) or
  [`typing.ParamSpec`](https://docs.python.org/3/library/typing.html#typing.ParamSpec) when the string argument matches
  the assigned name.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python ok
  from typing import TypeVar

  T = TypeVar("T")


  def first[U](items: list[U]) -> U:
      return items[0]
  ```

  A lowercase name, mismatched string or different function named `TypeVar` does not qualify.

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

- **`l`, `I` and `O`**: Ruff's [ambiguous-variable-name](https://docs.astral.sh/ruff/rules/ambiguous-variable-name/),
  [ambiguous-class-name](https://docs.astral.sh/ruff/rules/ambiguous-class-name/) and
  [ambiguous-function-name](https://docs.astral.sh/ruff/rules/ambiguous-function-name/) report these letters on the
  names they check, so this rule leaves them to Ruff there. Import aliases, `match` captures and type parameters keep
  this rule's report.

  ```python ok
  l = 3
  ```

### Reporting

Each name is reported once per scope, at its first binding.

## Ruff overlap

Ruff's E741, E742 and E743 report `l`, `I` and `O` on variables, parameters, except names, classes and functions, with
no fix. This rule leaves those three letters to Ruff at those sites. It keeps every other single letter, and the `l`,
`I` and `O` that Ruff does not check: import aliases, `match` captures and lowercase type parameters.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rules
  [E741](https://docs.astral.sh/ruff/rules/ambiguous-variable-name/),
  [E742](https://docs.astral.sh/ruff/rules/ambiguous-class-name/) and
  [E743](https://docs.astral.sh/ruff/rules/ambiguous-function-name/), which report `l`, `I` and `O` and which this rule
  leaves to them; issue [#4866](https://github.com/astral-sh/ruff/issues/4866) asking for all single letters was closed
  as not planned.
- [flake8-variables-names](https://github.com/best-doctor/flake8-variables-names) reports single-letter names as
  [VNE001](https://github.com/best-doctor/flake8-variables-names/blob/master/flake8_variables_names/checker.py), but
  allows `i`, `_` and `T` by default. This rule flags `for i` loop targets but allows `T` only when it declares a type
  parameter.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports short names as
  [WPS111](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/naming.html#wemake_python_styleguide.violations.naming.TooShortNameViolation),
  with a configurable minimum length. It also checks module and attribute names, which this rule does not list.
