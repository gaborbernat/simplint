Flags a class whose instances hold more attributes than the limit, which is 12 by default. It counts the attributes
its methods set on the instance and a dataclass's fields.

## Why is this bad?

A class that holds many attributes often holds several concerns, each a group of attributes that change together and
that a few methods use. Every method can read and change every attribute, so a reader has to check all of them to know
which ones a change touches. Smaller classes, each holding one group and held by the outer class, name the concerns and
limit what each method can reach. pylint's
[too-many-instance-attributes](https://pylint.readthedocs.io/en/stable/user_guide/messages/refactor/too-many-instance-attributes.html)
(R0902) reports the same smell.

## Example

```toml config
[tool.simplint.classes]
max-instance-attributes = 3
```

```python report
class Order:
    def __init__(self, street: str, city: str, total: int, currency: str) -> None:
        self.street = street
        self.city = city
        self.total = total
        self.currency = currency
```

Use instead:

```python ok
from dataclasses import dataclass


@dataclass(frozen=True)
class Address:
    street: str
    city: str


@dataclass(frozen=True)
class Money:
    total: int
    currency: str


class Order:
    def __init__(self, address: Address, price: Money) -> None:
        self.address = address
        self.price = price
```

## Configuration

- [`classes.max-instance-attributes`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#classes)
  in `[tool.simplint]`: the most attributes a class's instances may hold. A class holding more is reported, such as
  `max-instance-attributes = 4` under `[tool.simplint.classes]`

  ```toml config
  [tool.simplint.classes]
  max-instance-attributes = 4
  ```

  ```python ok
  class Order:
      def __init__(self, street: str, city: str, total: int, currency: str) -> None:
          self.street = street
          self.city = city
          self.total = total
          self.currency = currency
  ```

## What it does

### Active on

Each class is reported at its name. An attribute counts once, however many methods set it.

- **Attributes set on `self` in any method**: not only in `__init__`.

  ```toml config
  [tool.simplint.classes]
  max-instance-attributes = 1
  ```

  ```python report
  class Connection:
      def __init__(self, host: str) -> None:
          self.host = host

      def open(self) -> None:
          self.socket = object()
  ```

- **A dataclass's fields**: annotated class attributes, leaving out `ClassVar` ones and the `KW_ONLY` marker.

  ```toml config
  [tool.simplint.classes]
  max-instance-attributes = 1
  ```

  ```python report
  from dataclasses import dataclass


  @dataclass
  class Point:
      x: int
      y: int
  ```

### Exceptions

- **Attributes set in a class method**: they belong to the class, not its instances.

  ```toml config
  [tool.simplint.classes]
  max-instance-attributes = 1
  ```

  ```python ok
  class Counter:
      def __init__(self) -> None:
          self.count = 0

      @classmethod
      def reset(cls) -> None:
          cls.total = 0
  ```

- **`ClassVar` annotations and the `KW_ONLY` marker of a dataclass**: neither is an instance field.

  ```toml config
  [tool.simplint.classes]
  max-instance-attributes = 1
  ```

  ```python ok
  from dataclasses import KW_ONLY, dataclass
  from typing import ClassVar


  @dataclass
  class Point:
      registry: ClassVar[list[str]] = []
      _: KW_ONLY
      x: int
  ```

- **A class at the limit**: the count must be more than the limit.

  ```toml config
  [tool.simplint.classes]
  max-instance-attributes = 2
  ```

  ```python ok
  class Pair:
      def __init__(self, first: int, second: int) -> None:
          self.first = first
          self.second = second
  ```

### Fix

No fix is offered: grouping attributes into classes is a decision about the design.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not implemented; pylint R0902 is listed in
  [#970](https://github.com/astral-sh/ruff/issues/970) as waiting on [#1774](https://github.com/astral-sh/ruff/issues/1774).
- [pylint](https://github.com/pylint-dev/pylint) reports a class over `max-attributes`, 7 by default, as
  [R0902](https://pylint.readthedocs.io/en/stable/user_guide/messages/refactor/too-many-instance-attributes.html), with
  no fix.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports too many public
  instance attributes as
  [WPS230](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/complexity.html#wemake_python_styleguide.violations.complexity.TooManyPublicAttributesViolation),
  with no fix.
