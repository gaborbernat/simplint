Flags a method whose only statement returns the same method of `super()`, called with the method's own arguments. The
parent's method already does that job, so the override adds only a name to read.

## Why is this bad?

A method that passes every argument on to the parent and returns the result changes nothing. A reader still has to check
that it does nothing else, and the class carries a second copy of the parent's signature that can drift from it.
Deleting the method lets the subclass inherit the parent's method.

## Example

```python report
class Base:
    def load(self, name):
        return name.encode()


class Store(Base):
    def load(self, name):
        return super().load(name)
```

Use instead:

```python ok
class Base:
    def load(self, name):
        return name.encode()


class Store(Base):
    pass
```

## What it does

### Active on

A method of a module-level class is reported when its body, apart from a docstring, is a single
`return super().name(...)` with the method's own name. The call must pass the parameters after the receiver as they were
given: positional parameters in order, `*args` as `*args`, keyword-only parameters as `name=name`, and `**kwargs` as
`**kwargs`.

The rule reads the method `super()` reaches, so the class must have a single base class that the module defines at its
top level. When that class does not define the method, its own single base is read the same way. The method found must
be a plain `def` taking the same parameters under the same names, without defaults.

- **A method that forwards its whole signature**: positional, variadic and keyword-only parameters all pass on.

  ```python report
  class Base:
      def save(self, name, *args, flag, **kwargs):
          return name


  class Store(Base):
      def save(self, name, *args, flag, **kwargs):
          return super().save(name, *args, flag=flag, **kwargs)
  ```

  ```python fixed
  class Base:
      def save(self, name, *args, flag, **kwargs):
          return name


  class Store(Base):
      pass
  ```

- **A method the grandparent defines**: a base class without the method passes the lookup on to its own base.

  ```python report
  class Base:
      def load(self, name):
          return name.encode()


  class Cache(Base):
      pass


  class Store(Cache):
      def load(self, name):
          return super().load(name)
  ```

- **A forwarder with an annotation**: reported, but not fixed, since the annotations are the types the subclass declares
  for the method.

  ```python report
  class Base:
      def load(self, name):
          return name.encode()


  class Store(Base):
      def load(self, name: str) -> bytes:
          return super().load(name)
  ```

### Exceptions

- **A base class the module imports**, or more than one base class: the method `super()` reaches is not in the module,
  and it may be abstract, which the forwarder overrides to make the class concrete.

  ```python ok
  from collections.abc import Iterable


  class Items(Iterable):
      def __iter__(self):
          return super().__iter__()
  ```

- **A parent's method of another signature**: a forwarder that drops a parameter with a default, renames a parameter or
  changes its kind rejects calls that the parent's method takes, so deleting it changes what callers may pass.

  ```python ok
  class Base:
      def load(self, name, encoding="utf-8"):
          return name.encode(encoding)


  class Store(Base):
      def load(self, name):
          return super().load(name)
  ```

- **A parent's method with a decorator**, such as `@abstractmethod`: the forwarder of an abstract method makes the class
  concrete.

  ```python ok
  from abc import ABC, abstractmethod


  class Base(ABC):
      @abstractmethod
      def load(self, name):
          return name.encode()


  class Store(Base):
      def load(self, name):
          return super().load(name)
  ```

- **`__eq__` and `__hash__`**: a class that defines `__eq__` without `__hash__` gets `__hash__ = None`, so a forwarder
  of either keeps the pair the way the class wants it.

  ```python ok
  class Base:
      def __eq__(self, other):
          return self is other

      def __hash__(self):
          return id(self)


  class Store(Base):
      def __eq__(self, other):
          return super().__eq__(other)
  ```

- **A method with a default value**: a call that leaves the argument out takes the default from the method, and deleting
  the method would take the parent's default instead.

  ```python ok
  class Base:
      def load(self, name):
          return name.encode()


  class Store(Base):
      def load(self, name="cache"):
          return super().load(name)
  ```

- **A method with a decorator**: a decorator such as `@staticmethod`, `@property` or `@override` changes what the method
  is or what it declares, so removing it changes behavior or typing.

  ```python ok
  class Base:
      def load(self):
          return b""


  class Store(Base):
      @property
      def load(self):
          return super().load()
  ```

- **A method with a comment on the line above it**: the comment says why the method is there, such as a workaround for a
  type checker, so deleting it would undo that.

  ```python ok
  class Base:
      def load(self, name):
          return name.encode()


  class Store(Base):
      # Keeps the type checker from reporting the parent's loader.
      def load(self, name):
          return super().load(name)
  ```

- **A method with a comment in its body**: deleting the method would delete the comment with it.

  ```python ok
  class Base:
      def load(self, name):
          return name.encode()


  class Store(Base):
      def load(self, name):
          # Keeps the type checker from reporting the parent's loader.
          return super().load(name)
  ```

- **A call that renames, reorders or adds an argument**: the method does more than forward.

  ```python ok
  class Base:
      def load(self, name):
          return name.encode()


  class Store(Base):
      def load(self, name):
          return super().load(name.strip())
  ```

- **A call whose result is discarded**: the method returns `None`, and the parent's result would be returned instead.

  ```python ok
  class Base:
      def load(self, name):
          return name.encode()


  class Store(Base):
      def load(self, name):
          super().load(name)
  ```

- **A class with a decorator or a metaclass**, which may build or replace its methods.

  ```python ok
  from dataclasses import dataclass


  class Base:
      def load(self, name):
          return name.encode()


  @dataclass
  class Store(Base):
      def load(self, name):
          return super().load(name)
  ```

- **A name bound more than once in the class body**: the method that counts is the last binding, and deleting one of the
  others would change it. Imports and bindings under an `if` or `try` count too.

  ```python ok
  class Base:
      def load(self, name):
          return name.encode()


  class Store(Base):
      def load(self, name):
          return super().load(name)

      load = None
  ```

- **A name the class body reads**: once the method is gone, the class body looks the name up in the module, so a read
  such as `label = load` in the class body would fail or pick up another value. A call in another method's body runs
  later, in its own scope, so it does not count.

  ```python ok
  class Base:
      def load(self, name):
          return name.encode()


  class Store(Base):
      def load(self, name):
          return super().load(name)

      label = load
  ```

- **A class nested in a function or another class**: its base name may refer to a local class rather than the module's.

  ```python ok
  class Base:
      def load(self, name):
          return name.encode()


  def build():
      class Store(Base):
          def load(self, name):
              return super().load(name)

      return Store
  ```

- **An `async` method**: the parent's coroutine may differ from the one a forwarder returns, so the rule leaves it.

  ```python ok
  class Base:
      async def load(self, name):
          return name.encode()


  class Store(Base):
      async def load(self, name):
          return super().load(name)
  ```

### Fix

The fix deletes the method, along with the blank lines after it. A class left with no statement gets a `pass`. The fix
is offered only for methods without annotations.

The fix is safe: the parent's method takes the same arguments under the same names and is not abstract, and Python looks
up the deleted method's name in the same class order that `super()` follows, so every call reaches the same code.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issues [#970](https://github.com/astral-sh/ruff/issues/970)
  (pylint W0246 useless-parent-delegation) and [#3845](https://github.com/astral-sh/ruff/issues/3845) (WPS612).
- [pylint](https://github.com/pylint-dev/pylint) reports the same forwarding overrides as
  [useless-parent-delegation](https://pylint.readthedocs.io/en/latest/user_guide/messages/warning/useless-parent-delegation.html),
  with no fix. This rule's fix deletes the method when it has no annotations.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports overwritten methods
  that only repeat the parent's as
  [WPS612](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/oop.html#wemake_python_styleguide.violations.oop.UselessOverwrittenMethodViolation),
  with no fix.
