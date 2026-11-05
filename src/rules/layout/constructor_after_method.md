Flags constructors that a method sits above in a class body. `__new__`, `__init__`, `__post_init__` and
`__init_subclass__` come first, ahead of the other methods, so the reader starts where the instance is made.

## Why is this bad?

A class is read to learn how its instances come to be and what they hold from the start. A constructor placed after a
dozen methods sends the reader past all of them to find it, and the methods then read as if the instance already
existed. Putting the constructors first follows the order of an object's life: it is created, then used.

## Example

```python report
class Parser:
    def load(self, text: str) -> list[str]:
        return [line.strip() for line in text.splitlines()]

    def __init__(self, sep: str = ",") -> None:
        self.sep = sep
```

Use instead:

```python ok
class Parser:
    def __init__(self, sep: str = ",") -> None:
        self.sep = sep

    def load(self, text: str) -> list[str]:
        return [line.strip() for line in text.splitlines()]
```

## What it does

The rule checks each class body that defines a method. Every constructor defined below the first method that is not a
constructor is reported, and its message names that method.

### Exceptions

A constructor that moving would change is still reported, without a fix, when:

- its decorators or default values read a name the body binds between the first method and the constructor, such as a
  class attribute that a default reads:

  ```python report
  class Parser:
      def load(self, text: str) -> list[str]:
          return text.split(self.SEP)

      SEP = ","

      def __init__(self, sep: str = SEP) -> None:
          self.sep = sep
  ```

- the constructor's name is bound more than once in the body, such as by an assignment. Its `@overload` variants do not
  count as bindings of their own.

  ```python report
  class Parser:
      def load(self, text: str) -> list[str]:
          return text.split(",")

      def __init__(self) -> None:
          pass

      __init__ = None
  ```

- the class body reads the constructor's name as it runs, outside a method.

Annotations count as reads of a constructor's decorators and defaults when Python evaluates them as the `def` runs: a
decorated constructor's annotations always are, and the others' are unless the module has lazy annotations, as
[`definition-before-use`](SMP201.html) describes.

[`definition-before-use`](SMP201.html) never moves a constructor below a method that calls it, so a method that calls
`self.__init__()` stays below the constructor and the two rules agree.

### Fix

The fix moves the constructors above the first method, keeping their order. Each constructor carries its decorators and
the comment lines directly above it, as the fix of `definition-before-use` does. Overloads of a constructor move
together: when one variant is an exception, none of them moves.

- Only the first report of a class body carries the fix, since that one fix reorders every constructor it moves.

  ```python report
  class Parser:
      def load(self, text: str) -> list[str]:
          return text.split(self.sep)

      def __init__(self, sep: str = ",") -> None:
          self.sep = sep
  ```

  ```python fixed
  class Parser:
      def __init__(self, sep: str = ",") -> None:
          self.sep = sep

      def load(self, text: str) -> list[str]:
          return text.split(self.sep)
  ```

The fix is safe when the rewritten span holds no class and no decorated method. **Unsafe fix:** a span that shifts a
class or a decorated method, such as an `@overload` variant or a decorated constructor, because
[decorators](https://docs.python.org/3/glossary.html#term-decorator) and class creation then run in a different order.

No fix is offered when the span holds a comment that is not attached to a definition, or when two statements share a
line. A constructor that is an exception keeps its report without a fix, and the other constructors of the body still
move.
