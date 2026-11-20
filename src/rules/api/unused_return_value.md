Flags functions that production code calls but whose return value every production caller discards.

## Why is this bad?

A return value suggests that callers need it. If callers rely only on a side effect, returning `None` makes that clear.
Python's [`list.sort()`](https://docs.python.org/3/faq/design.html#why-doesn-t-list-sort-return-the-sorted-list) follows
this convention for an in-place change.
[Command-query separation](https://martinfowler.com/bliki/CommandQuerySeparation.html) describes the distinction between
changing state and querying it.

## Example

```python context file=app/service.py
from app.store import save_order


def place_order(order: str) -> None:
    save_order(order)
```

```python report file=app/store.py
def save_order(order: str) -> str:
    print("saved", order)
    return order
```

Use instead:

```python ok file=app/store.py
def save_order(order: str) -> None:
    print("saved", order)
```

## What it does

For each function that returns a value, the rule checks every call the run makes. A standalone call such as
`save_order(order)` discards the result. The rule counts a call inside another expression as a read. The function is
reported when production code calls it and no production call reads the value, whether or not tests read it.

### Active on

- **Module-level functions** whose calls in production code all drop the value.

  ```python context file=app/service.py
  from app.store import save_order


  def place_order(order: str) -> None:
      save_order(order)
  ```

  ```python report file=app/store.py
  def save_order(order: str) -> str:
      print("saved", order)
      return order
  ```

- **Functions whose value only tests read**: a test that reads the value does not count as a production use.

  ```python report file=app/store.py
  def load_total(order: str) -> int:
      return len(order)
  ```

  ```python context file=app/service.py
  from app.store import load_total


  def place_order(order: str) -> None:
      load_total(order)
  ```

  ```python context file=tests/test_store.py
  from app.store import load_total


  def test_total() -> None:
      assert load_total("abc") == 3
  ```

- **Methods called through `self`**: a method of a module-level class without bases, when every read of its name in the
  run is a `self.name(...)` call inside the class.

  ```python report
  class Writer:
      def write(self, data: str) -> int:
          return len(data)

      def commit(self, data: str) -> None:
          self.write(data)
  ```

### Exceptions

The rule skips test modules and modules that another file star-imports. It also skips a module-level function if its
name is rebound.

- **Functions whose value a production call reads**: one reading call keeps the function.

  ```python context file=app/service.py
  from app.store import save_order


  def place_order(order: str) -> int:
      return len(save_order(order))
  ```

  ```python ok file=app/store.py
  def save_order(order: str) -> str:
      print("saved", order)
      return order
  ```

- **Functions with no return value, or that return only `self`**: a function that returns nothing, or only `None`, has
  no value to read, and a method that returns `self` lets callers chain calls, a style of its interface.

  ```python ok
  class Query:
      def where(self, clause: str) -> "Query":
          print(clause)
          return self

      def run(self) -> None:
          self.where("a = 1")


  Query().run()
  ```

  ```python ok
  def save_order(order: str) -> None:
      print("saved", order)


  save_order("a")
  ```

- **Functions only tests call**: a function production code never calls is test support, which
  [test-only-code](SMP408.html) can report.

  ```python ok file=app/store.py
  def load_total(order: str) -> int:
      return len(order)
  ```

  ```python context file=tests/test_store.py
  from app.store import load_total


  def test_total() -> None:
      assert load_total("abc") == 3
  ```

- **Functions with no call**: [unused-definition](SMP409.html) checks for definitions with no known use.

  ```python ok
  def save_order(order: str) -> str:
      return order
  ```

- **Functions used other than as a call**: passed as a value, or named by a dotted string or an entry point, where the
  calls that reach them are not all visible.

  ```python ok file=app/store.py
  def save_order(order: str) -> str:
      return order


  HANDLERS = [save_order]
  ```

  ```python context file=app/service.py
  from app.store import save_order


  def place_order(order: str) -> None:
      save_order(order)
  ```

- **Program entry points** may use a return value as the exit status, for example with
  [`sys.exit(main())`](https://docs.python.org/3/library/sys.html#sys.exit). The rule skips a function called from a
  main guard or anywhere in `__main__.py`.

  ```python ok
  def main() -> int:
      return 0


  if __name__ == "__main__":
      main()
  ```

- **Modules only tests import**: a helper module outside the test directories that only test modules import, directly or
  through other such modules, serves the tests, so its calls are test code.

  ```python ok file=testing/util.py
  def git_commit(message: str) -> str:
      return message


  def commit_all() -> None:
      git_commit("all")
  ```

  ```python context file=tests/test_git.py
  from testing.util import commit_all, git_commit


  def test_commit() -> None:
      commit_all()
      assert git_commit("a") == "a"
  ```

- **Decorated functions** can have callers that never appear in the source. The rule still checks functions decorated
  with [`functools.cache`](https://docs.python.org/3/library/functools.html#functools.cache), `lru_cache`, `wraps`,
  [`contextlib.contextmanager`](https://docs.python.org/3/library/contextlib.html#contextlib.contextmanager),
  `asynccontextmanager`, [`abc.abstractmethod`](https://docs.python.org/3/library/abc.html#abc.abstractmethod),
  `classmethod`, or `final` and `override` from [`typing`](https://docs.python.org/3/library/typing.html#typing.final)
  or `typing_extensions`.

  ```python ok
  from atexit import register


  @register
  def save_order(order: str) -> str:
      return order


  save_order("a")
  ```

- **Dunder methods**: Python calls them itself, not by name.

  ```python ok
  class Order:
      def __str__(self) -> str:
          return "order"


  print(Order())
  ```

- **Public API**: the public functions and methods of a library, and the names `__all__` lists, may be called by code
  outside the run.

  ```toml config
  [build-system]
  requires = ["setuptools"]

  [project]
  name = "mylib"
  ```

  ```python ok file=mylib/api.py
  def save_order(order: str) -> str:
      return order
  ```

  ```python context file=mylib/service.py
  from mylib.api import save_order


  def place_order(order: str) -> None:
      save_order(order)
  ```

- **Methods with uncertain callers** include methods of classes with bases, class keywords such as `metaclass=`, or
  known subclasses. The rule also skips a method if the project assigns an attribute of the same name or reads it
  through another object.

  ```python ok
  class Base:
      pass


  class Writer(Base):
      def write(self, data: str) -> int:
          return len(data)

      def commit(self, data: str) -> None:
          self.write(data)


  Writer().commit("a")
  ```

### Fix

A caller may need to use the result, or the function may need to stop returning it. Removing the return expression could
also remove a side effect. The rule offers no fix.
