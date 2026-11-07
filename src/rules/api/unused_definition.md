Flags functions, classes, methods and class attributes that nothing in the run uses, so the program never reaches them.

## Why is this bad?

Code nothing calls still has to be read, kept compiling and updated when its neighbors change. Readers spend time
working out whether it matters, and the test suite rarely covers it, so it drifts. Removing it keeps the module to what
the program does.

## Example

```python report
def format_name(first: str, last: str) -> str:
    return f"{first} {last}"


print("ready")
```

Use instead:

```python ok
print("ready")
```

## What it does

A name counts as used when the run imports it, reads it through a module, names it in a dotted string or a doctest,
refers to it elsewhere in its own module, or spells it in a string of its module, as a string annotation does. A read of
`obj.name` anywhere in the run counts as a use of every function, class, method and class attribute named `name`, since
the object may be a module the run loads under a computed name. The project's documentation counts too: a definition a
Sphinx page names is used.

### Active on

- **Module-level functions and classes** that no reference in the run reaches.

  ```python report
  def format_name(first: str, last: str) -> str:
      return f"{first} {last}"


  print("ready")
  ```

  ```python report
  class Record:
      pass


  print("ready")
  ```

- **Methods** of a module-level class without bases or decorators, when no `obj.name` read of the method name exists in
  the run.

  ```python report
  class Client:
      def send(self, data: str) -> None:
          print(data)


  print(Client())
  ```

- **Class attributes**: plain `NAME = value` assignments in the same kind of class, which nothing reads.

  ```python report
  class Client:
      RETRIES = 3

      def send(self) -> None:
          print("sent")


  print(Client().send())
  ```

### Exceptions

- **Names used elsewhere in the run**: a function imported by another module, or called from the module itself, is used.

  ```python ok file=app/format.py
  def format_name(first: str, last: str) -> str:
      return f"{first} {last}"
  ```

  ```python context file=app/main.py
  from app.format import format_name


  print(format_name("Ada", "Lovelace"))
  ```

- **Names read by a dotted string**: a `"pkg.module.name"` string, such as the target of `mock.patch`, counts as a read.

  ```python ok file=app/format.py
  def format_name(first: str, last: str) -> str:
      return f"{first} {last}"
  ```

  ```python context file=tests/test_format.py
  from unittest import mock


  def test_format() -> None:
      with mock.patch("app.format.format_name"):
          pass
  ```

- **Names listed in `__all__`**: the module exports them, so the run may use them from outside.

- **Names doctests and unresolved modules use**: a `>>>` example in a docstring runs code the imports do not show, and a
  module the run cannot resolve may import a name of the run under another name, as CPython's frozen modules do.

  ```python ok file=app/markup.py
  class _Escaped:
      """Escapes text.

      >>> from app.markup import _Escaped
      >>> _Escaped()  # doctest: +SKIP
      """
  ```

  ```python ok
  __all__ = ["format_name"]


  def format_name(first: str, last: str) -> str:
      return f"{first} {last}"
  ```

- **Names a class body or a string uses**: a class attribute such as `x = property(_get_x)` reads the method, and a
  string annotation reads the class it names.

  ```python ok
  from typing import cast


  class _Entry:
      pass


  class Point:
      def _get_x(self) -> int:
          return 1

      x = property(_get_x)


  print(Point().x, cast("list[_Entry]", []))
  ```

- **Private protocols and TypedDicts Ruff reports**: when the project's Ruff configuration selects
  [PYI046](https://docs.astral.sh/ruff/rules/unused-private-protocol/) or
  [PYI049](https://docs.astral.sh/ruff/rules/unused-private-typed-dict/), Ruff reports a private `Protocol` or
  `TypedDict` class that nothing in its file reads, so this rule leaves it to Ruff.

  ```toml config
  [tool.ruff.lint]
  select = ["PYI046", "PYI049"]
  ```

  ```python ok
  from typing import Protocol, TypedDict


  class _Runner(Protocol):
      def run(self) -> None: ...


  class _Row(TypedDict):
      a: int


  print("ready")
  ```

- **Names read off an object**: an `obj.name` read may reach a module, so it keeps every definition named `name`.

  ```python ok file=app/sysroot.py
  def _default_sysroot() -> str:
      return "/"
  ```

  ```python context file=app/main.py
  import importlib

  module = importlib.import_module("app.sysroot")
  print(module._default_sysroot())
  ```

- **Names the documentation gives**: a `.rst` or `.md` file under the project names a definition with a Sphinx role such
  as `:func:`, an autodoc directive such as `.. autofunction::`, or the `:module:` and `:func:` options of a
  sphinx-argparse or sphinx-argparse-cli directive. An `.. automodule::` with `:members:` documents the members Sphinx
  renders: the names its module's `__all__` lists, or else its public names, without the ones `:exclude-members:` names,
  and with the private and special names when `:private-members:` or `:special-members:` is set. Its classes' members
  are documented too, and so is every member of an `.. autoclass::` with `:members:`. Names match by their dotted
  ending, so a page under `.. currentmodule::`, or one whose `conf.py` puts another directory on `sys.path`, still
  matches. The files are read from the directory of each `pyproject.toml`, outside hidden directories and the paths Ruff
  excludes.

  ```toml config
  [project]
  name = "app"
  ```

  ```python ok file=app/cli.py
  from argparse import ArgumentParser


  def build_parser() -> ArgumentParser:
      return ArgumentParser(prog="app")
  ```

  ```rst context file=docs/cli.rst
  .. sphinx_argparse_cli::
     :module: app.cli
     :func: build_parser
  ```

  ```toml config
  [project]
  name = "app"
  ```

  ```python ok file=app/format.py
  def format_name(first: str, last: str) -> str:
      return f"{first} {last}"
  ```

  ```markdown context file=docs/api.md
  Call {py:func}`~app.format.format_name` to join the parts.
  ```

- **A library's public API**: a name a public module offers may be used by code outside the run.

  ```toml config
  [build-system]
  requires = ["setuptools"]

  [project]
  name = "mylib"
  ```

  ```python ok file=mylib/api.py
  def format_name(first: str, last: str) -> str:
      return f"{first} {last}"
  ```

- **Decorated definitions**: a decorator may register the definition with something outside the run, such as a plugin
  list or a framework.

  ```python ok
  from functools import cache


  @cache
  def format_name(first: str, last: str) -> str:
      return f"{first} {last}"
  ```

- **Framework hooks**: `setup` and `teardown` functions, and `pytest_*` hooks, are called by Sphinx or pytest by name,
  and so are the hooks of the build backend that `build-backend` names in `[build-system]`, which a build frontend
  calls.

  ```python ok file=docs/conf.py
  def setup(app: object) -> None:
      print(app)
  ```

- **Dunder and sunder names** such as `__repr__`, which the interpreter calls without a name in the source, and
  `_repr_html_` or `_missing_`, which Jupyter and `enum` call by name.

  ```python ok
  class Record:
      def __repr__(self) -> str:
          return "Record"

      def _repr_html_(self) -> str:
          return "<b>Record</b>"


  print(Record())
  ```

- **Methods and attributes of classes with bases**: a base class may call the method, or a framework may. Classes with a
  decorator are left alone for the same reason.

  ```python ok
  class Base:
      pass


  class Client(Base):
      def send(self, data: str) -> None:
          print(data)


  print(Client())
  ```

- **Test modules**: a test module's helpers belong to the tests, and pytest may call them by name.

  ```python ok file=tests/test_helpers.py
  def make_user() -> str:
      return "alice"
  ```

- **Computed attribute reads**: a read under a name computed at run time, such as `getattr(obj, name)`, `vars(obj)` or
  `dir(obj)`, may reach any name of the module it sits in, and any public name of the run. A private name is therefore
  kept only when its own file makes such a read. A name built on a literal start, as `getattr(self, "_handle_" + kind)`
  or `getattr(self, f"_handle_{kind}")` builds for a dispatch table, reaches only the names with that start, wherever
  they are defined; a start of underscores alone picks out no family, so it counts as any name.

  ```python ok
  class Client:
      def send(self) -> None:
          print("sent")


  def call(client: Client, name: str) -> object:
      return getattr(client, name)


  print(call(Client(), "send"))
  ```

  ```python ok file=app/handlers.py
  class Handlers:
      def _handle_text(self) -> None:
          print("text")


  print(Handlers())
  ```

  ```python context file=app/dispatch.py
  def dispatch(handlers: object, kind: str) -> object:
      return getattr(handlers, "_handle_" + kind)
  ```

  A name outside the family is still judged, in the file of the read as well:

  ```python report
  class Handlers:
      def _handle_text(self) -> None:
          print("text")

      def _render(self) -> None:
          print("render")


  def dispatch(handlers: Handlers, kind: str) -> object:
      return getattr(handlers, f"_handle_{kind}")


  print(dispatch(Handlers(), "text"))
  ```

### Fix

The rule has no fix. Deleting a definition can strand the imports it used, and it changes the program when something
outside the run reads the name in a way simplint cannot see, such as a plugin loaded by name. The author decides what to
remove.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rules
  [PYI046](https://docs.astral.sh/ruff/rules/unused-private-protocol/) and
  [PYI049](https://docs.astral.sh/ruff/rules/unused-private-typed-dict/), which report a private `Protocol` or
  `TypedDict` unused in its own file and which this rule leaves to them; unused definitions across the project are open
  issue [#872](https://github.com/astral-sh/ruff/issues/872).
- [vulture](https://github.com/jendrikseipp/vulture) reports unused functions, classes, methods and attributes across
  the files it is given, with a confidence score and no fix.
- [deadcode](https://github.com/albertas/deadcode) reports the same definitions as
  [DC02](https://github.com/albertas/deadcode) to DC04, and its `--fix` option deletes them. This rule offers no fix,
  since a deletion can strand the imports the definition used.
