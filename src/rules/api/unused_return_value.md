Flags a function whose return value no production call reads: its callers drop the value, or only tests read it.

## Why is this bad?

A `return` that no caller reads still tells readers the value matters. They look for the caller that consumes it and
find none. Returning nothing says the function acts through its side effects.
[`list.sort()`](https://docs.python.org/3/faq/design.html#why-doesn-t-list-sort-return-the-sorted-list) follows that
convention to remind callers it changed the list in place, and
[command-query separation](https://martinfowler.com/bliki/CommandQuerySeparation.html) draws the same split between
methods that change state and methods that answer questions. The function also stops building a value that is thrown
away.

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

For each function that returns a value, the rule checks every call the run makes. A call on a line of its own drops the
value, as `save_order(order)` does, and a call inside an expression reads it. The function is reported when production
code calls it and no production call reads the value, whether or not tests read it.

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

- **Functions only tests call**: a function production code never calls is test support, which `test-only-code` reports.

  ```python ok file=app/store.py
  def load_total(order: str) -> int:
      return len(order)
  ```

  ```python context file=tests/test_store.py
  from app.store import load_total


  def test_total() -> None:
      assert load_total("abc") == 3
  ```

- **Functions with no call**: nothing calls them, so the rule has no call to judge. `unused-definition` reports them.

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

- **A program's entry**: a call from an `if __name__ == "__main__":` block or a `__main__.py` hands the value to the
  command line, which may use it as the exit status.

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

- **Decorated functions**: a decorator may wrap the function or use its value, so its calls may not be the ones written.
  A decorator that passes each call on unchanged keeps the function judged: `functools.cache`, `lru_cache` and `wraps`,
  `contextlib.contextmanager`, `abc.abstractmethod`, `classmethod`, and `typing.final` and `override`.

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

- **Methods of classes with bases, or whose name is read elsewhere**: a subclass may override the method, and a read of
  the name through another object hides which method it reaches.

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

The rule has no fix. Dropping the `return` changes the signature that tests and outside callers of a library may read,
and the value may be the only thing a caller keeps. The author decides whether the caller or the function should change.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
