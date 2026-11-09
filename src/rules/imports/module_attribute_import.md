Flags `module.Name` attribute access on a module of this project, where `from module import Name` would bind the same
object.

## Why is this bad?

Reaching a name through its module makes the reader trace where the module came from at every use, and it repeats the
module name in each line. Importing the name once states the dependency at the top of the file.

## Example

```python context file=pkg/__init__.py

```

```python context file=pkg/findings.py
class DbWriter:
    pass
```

```python report file=pkg/consumer.py
from . import findings

writer = findings.DbWriter()
```

Use instead:

```python context file=pkg/__init__.py

```

```python context file=pkg/findings.py
class DbWriter:
    pass
```

```python ok file=pkg/consumer.py
from .findings import DbWriter

writer = DbWriter()
```

## Configuration

- [`module-attributes`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#module-attributes):
  modules whose attributes the code reaches as `module.Name`, by absolute dotted name, such as a module that tests patch
  attribute by attribute. The default is `[]`.

  ```toml config
  [tool.simplint.imports]
  module-attributes = ["pkg.findings"]
  ```

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/findings.py
  class DbWriter:
      pass
  ```

  ```python ok file=pkg/consumer.py
  from . import findings

  writer = findings.DbWriter()
  ```

## What it does

### Active on

- **Attribute access on a module of this project bound by `from . import module` or `from package import module`**: the
  module must resolve to a file of the run, so the attribute reads a name the file defines.

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/findings.py
  class DbWriter:
      pass
  ```

  ```python report file=pkg/consumer.py
  from . import findings

  writer = findings.DbWriter()
  ```

- **Attribute access on a module bound by `import package.module as name`**, which binds the module like the form above.

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/findings.py
  class DbWriter:
      pass
  ```

  ```python report file=pkg/consumer.py
  import pkg.findings as findings

  writer = findings.DbWriter()
  ```

- **Private attributes of imported modules**, such as `main._compute`, including when Ruff selects `PLC2701`. Only a
  module that resolves to a file of this project is checked.

  ```python context file=pkg/main.py
  def _compute() -> int:
      return 1
  ```

  ```python report file=pkg/consumer.py
  from . import main

  result = main._compute()
  ```

### Exceptions

- **Third-party and standard-library modules**: only modules that resolve to a file of the run count, so `json` stays as
  it is.

  ```python ok
  import json as jsonlib

  text = jsonlib.dumps([])
  ```

- **The project's own top-level package under another name**: `import pkg as pd` reads the package's names through its
  alias, as `pd.NAT` does, which is how the package is meant to be used. A read through a submodule, such as
  `import pkg.core.common as com`, is still reported, and so is an alias equal to the package name, `import pkg as pkg`.

  ```python context file=pkg/__init__.py
  NAT = object()
  ```

  ```python ok file=pkg/consumer.py
  import pkg as pd

  writer = pd.NAT
  ```

- **Modules listed in
  [`module-attributes`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#module-attributes)**:
  the listed module is left alone, as in the configuration example above.

  ```toml config
  [tool.simplint.imports]
  module-attributes = ["pkg.findings"]
  ```

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/findings.py
  class DbWriter:
      pass
  ```

  ```python ok file=pkg/consumer.py
  from . import findings

  writer = findings.DbWriter()
  ```

- **A module used as a value**: passed to a function, assigned or returned, as in `getattr(findings, "DbWriter")` or
  `importlib.reload(findings)`, the module is an object of its own, so the file keeps it.

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/findings.py
  class DbWriter:
      pass
  ```

  ```python ok file=pkg/consumer.py
  from . import findings

  writer = getattr(findings, "DbWriter")
  ```

- **Attributes stored or deleted**: `findings.DEBUG = True` and `del findings.OLD` change the module, which an import
  cannot do. A store in any file of the run patches that attribute the way `setattr` over a literal does, so the
  attribute is left alone while the module's other attributes are still reported. A name a function of the module
  rebinds through `global` changes it after the import has bound the value it had at load, so that name is not converted
  either.

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/findings.py
  DEBUG = False


  def enable() -> None:
      global DEBUG
      DEBUG = True
  ```

  ```python ok file=pkg/consumer.py
  from . import findings

  print(findings.DEBUG)
  ```

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/findings.py
  class DbWriter:
      pass
  ```

  ```python ok file=pkg/consumer.py
  from . import findings

  findings.DEBUG = True
  del findings.OLD
  ```

- **Names the file binds itself**: when the file already binds the attribute name to something else, importing it would
  clash with that binding.

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/findings.py
  class DbWriter:
      pass
  ```

  ```python ok file=pkg/consumer.py
  from . import findings

  DbWriter = object
  writer = findings.DbWriter()
  ```

- **A name the module's own name shadows**: a parameter, local, loop or comprehension variable named like the module is
  that variable's value, so an attribute read off it does not reach the module. The real reads of the module are still
  reported.

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/findings.py
  class DbWriter:
      pass
  ```

  ```python ok file=pkg/consumer.py
  from . import findings


  def write(findings: object) -> object:
      return findings.DbWriter()
  ```

- **Module dunders such as `__name__`**: they describe the module they are read from, so importing them would read the
  wrong module.

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/findings.py
  class DbWriter:
      pass
  ```

  ```python ok file=pkg/consumer.py
  from . import findings

  NAME = findings.__name__
  ```

- **Modules that tests patch by attribute**: a dotted string such as `mocker.patch("pkg.findings.DbWriter")` names the
  attribute in the module, so the code keeps reaching it through the module, where the patch takes effect. The module's
  other attributes are still reported. A `setattr` in a loop over a literal tuple or list of modules patches the name on
  each of them the same way, until a statement rebinds the loop's name or a function's parameter shadows it.

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/findings.py
  class DbWriter:
      pass
  ```

  ```python ok file=pkg/consumer.py
  from . import findings

  writer = findings.DbWriter()
  ```

  ```python context file=tests/test_writer.py
  def test_writer(mocker):
      mocker.patch("pkg.findings.DbWriter")
  ```

### Fix

**Unsafe fix:** the fix rewrites the import to bind the names, so the object is bound when the module loads. A test that
patches the module attribute later no longer reaches the name. The fix replaces the import and every `findings.Name`
access in the file, and it is offered only when every use of the module is such an access to a name the module defines
and no computed write, such as `setattr(module, name, value)`, may set a name of the module.

```python context file=pkg/__init__.py
# A package.
```

```python context file=pkg/findings.py
class DbWriter:
    pass
```

```python report file=pkg/consumer.py
from . import findings

writer = findings.DbWriter()
```

```python fixed file=pkg/consumer.py
from .findings import DbWriter

writer = DbWriter()
```

A use the fix cannot rewrite keeps the report without a fix. Here `findings.__name__` is a module dunder, so `DbWriter`
is reported without a fix.

```python context file=pkg/__init__.py
# A package.
```

```python context file=pkg/findings.py
class DbWriter:
    pass
```

```python report file=pkg/consumer.py
from . import findings

writer = findings.DbWriter()
label = findings.__name__
```

A use that a local binding of the same name shadows keeps the report without a fix, since the bare name would resolve to
that binding. Here the type parameter `Item` shadows the module's `Item`.

```toml config
[project]
name = "demo"
requires-python = ">=3.12"
```

```python context file=pkg/__init__.py
# A package.
```

```python context file=pkg/findings.py
class Item:
    pass
```

```python report file=pkg/consumer.py
from . import findings


def make[Item](kind: type[Item]) -> object:
    return findings.Item()
```

A `lazy` import stays lazy after the fix, so the module still loads on first use of a name.

```toml config
[project]
name = "demo"
requires-python = ">=3.15"
```

```python context file=pkg/__init__.py
# A package.
```

```python context file=pkg/findings.py
class DbWriter:
    pass
```

```python report file=pkg/consumer.py
lazy from . import findings

writer = findings.DbWriter()
```

```python fixed file=pkg/consumer.py
lazy from .findings import DbWriter

writer = DbWriter()
```

## Private module attributes

The rule reports `module._name` when `module` resolves to a file in this project. It does not check attributes of
classes, functions or other values. Private names and private module paths receive no automatic fix: a replacement
import can cross a package boundary or violate
[private-import-path](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP310.html). Ruff's
[import-private-name](https://docs.astral.sh/ruff/rules/import-private-name/) (PLC2701) checks imports, not attribute
access, so selecting it does not suppress this report.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issues [#21025](https://github.com/astral-sh/ruff/issues/21025)
  and [#14070](https://github.com/astral-sh/ruff/issues/14070).
