Flags variables, parameters and attributes named with a placeholder word that says nothing about what they hold, such as
`foo` or `baz`. The words come from a list you can change.

## Why is this bad?

A name is the only description a value carries at each place it is used. `foo` leaves the reader to find where it is
assigned, and then to learn what it is. A name for the contents, such as `counts`, answers that at once.

## Example

```python report
def load(payload: bytes) -> dict[str, int]:
    foo = parse(payload)
    return foo
```

Use instead:

```python ok
def load(payload: bytes) -> dict[str, int]:
    counts = parse(payload)
    return counts
```

## Configuration

- [`naming.meaningless-names`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#naming) in
  `[tool.simplint]`: the words to report, `foo`, `baz`, `qux` and `quux` by default. Words such as `data`, `result`,
  `value` or `tmp` are left out: a parser's `feed(data)`, a `for key, value in` loop or a `tmp` directory reads well in
  context. A list replaces the default one, so a project that bans more words lists the defaults too.

  ```toml config
  [tool.simplint.naming]
  meaningless-names = ["foo", "baz", "tmp"]
  ```

  ```python report
  tmp = load()
  ```

## What it does

Reported names are the ones the list holds, compared as whole names, so `foo_count` is not `foo`. A word matches in
lower case, or in upper case as a constant spells it, with or without leading or trailing underscores.

### Active on

- **Variables**: names bound by assignment, `for` loops, `with ... as` and the walrus operator.

  ```python report
  foo = 3
  ```

  ```python report
  _FOO = 3
  ```

  ```python report
  for foo in items:
      print(foo)
  ```

- **Parameters** of a function or a lambda.

  ```python report
  def scale(factor: float, foo: float) -> float:
      return factor * foo
  ```

- **Exception and `match` names**: the name `except ... as` and a `case` capture binds.

  ```python report
  try:
      run()
  except OSError as foo:
      print(foo)
  ```

- **Attributes a method declares**: `self.name = ...` in a method, or `cls.name = ...` in a classmethod.

  ```python report
  class Cache:
      def __init__(self) -> None:
          self.foo = {}
  ```

### Exceptions

- **Overrides**: the parameters of a method decorated with
  [`@override`](https://docs.python.org/3/library/typing.html#typing.override) keep the names of the method they
  replace.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python ok
  from json import JSONEncoder
  from typing import override


  class Encoder(JSONEncoder):
      @override
      def default(self, foo: object) -> object:
          return str(foo)
  ```

- **Dunder methods**: the parameters of a method named with double underscores, such as `value` in `__setitem__`, keep
  the names of the protocol they implement.

  ```toml config
  [tool.simplint.naming]
  meaningless-names = ["value"]
  ```

  ```python ok
  class Store:
      def __setitem__(self, key: str, value: int) -> None:
          self.items = {key: value}
  ```

- **Attributes written through other objects**: `response.foo = ...` names an attribute the code does not choose.

  ```python ok
  def fill(response) -> None:
      response.foo = b""
  ```

- **Names outside the list**: a name that merely contains a listed word is a name with a meaning.

  ```python ok
  foo_count = load()
  ```

- **Class names**: `Foo` names a class, which the rule does not check where it is defined, so a `Foo = ...` that rebinds
  one is not reported either.

  ```python ok
  Foo = make_class()
  ```

### Reporting

Each name is reported once per scope, at its first binding, as [single-letter-name](SMP101.html) reports its names.

## Ruff overlap

Ruff's [ambiguous-variable-name](https://docs.astral.sh/ruff/rules/ambiguous-variable-name/) covers `l`, `O` and `I`
only. This rule covers a word list you choose, like pylint's
[disallowed-name](https://pylint.readthedocs.io/en/latest/user_guide/messages/convention/disallowed-name.html).

## Fix

No fix is offered, since a meaningful name cannot be derived from the code.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issues [#970](https://github.com/astral-sh/ruff/issues/970),
  [#3845](https://github.com/astral-sh/ruff/issues/3845) and [#3463](https://github.com/astral-sh/ruff/issues/3463).
- [flake8-variables-names](https://github.com/best-doctor/flake8-variables-names) reports placeholder names such as
  `data` as
  [VNE002](https://github.com/best-doctor/flake8-variables-names/blob/master/flake8_variables_names/checker.py). Its
  strict option adds more words.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports a fixed list of
  blacklisted names as
  [WPS110](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/naming.html#wemake_python_styleguide.violations.naming.WrongVariableNameViolation).
- [pylint](https://github.com/pylint-dev/pylint) reports the names in its
  [disallowed-name](https://pylint.readthedocs.io/en/latest/user_guide/messages/convention/disallowed-name.html) list.
  That list defaults to placeholders such as `foo` and `bar`, not words such as `data`.
