Flags attribute reads such as `findings.DbWriter` on imported modules. By default it checks modules in the project.

## Why is this bad?

`from findings import DbWriter` shows which part of `findings` the file uses and lets callers write `DbWriter()`
directly.

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

- [`module-attribute-scope`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#module-attributes):
  the modules the rule checks. `"project"` covers the modules of the run, `"stdlib"` the standard library,
  `"third-party"` every other module, and an absolute module name covers that module and its submodules, so a project
  can widen the rule one module at a time. The default is `["project"]`.

  ```toml config
  [tool.simplint.imports]
  module-attribute-scope = ["project", "stdlib"]
  ```

  ```python report
  import json

  text = json.dumps([])
  ```

## What it does

### Active on

- **Attribute access on a module of this project bound by `from . import module` or `from package import module`**: the
  module must resolve to a file in the run. A report can name an attribute that the file does not define.

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

- **Attribute access on a module outside the run bound by `import module` or `import module as name`**, once
  `module-attribute-scope` covers it. The run does not show which names such a module defines, so the report comes
  without a fix.

  ```toml config
  [tool.simplint.imports]
  module-attribute-scope = ["project", "requests"]
  ```

  ```python report
  import requests

  response = requests.get("https://example.com")
  ```

- **Private attributes of imported modules**, such as `main._compute`, including when Ruff selects `PLC2701`. The
  configured scope determines which modules are checked.

  ```python context file=pkg/main.py
  def _compute() -> int:
      return 1
  ```

  ```python report file=pkg/consumer.py
  from . import main

  result = main._compute()
  ```

### Exceptions

- **Third-party and standard-library modules under the default scope**: only modules that resolve to a file of the run
  count until `module-attribute-scope` lists more, so `json` stays as it is.

  ```python ok
  import json as jsonlib

  text = jsonlib.dumps([])
  ```

- **A module imported under its conventional alias**: Ruff's
  [`flake8-import-conventions`](https://docs.astral.sh/ruff/settings/#lint_flake8-import-conventions_aliases) asks for
  `import numpy as np`, `import pandas as pd` and the other aliases of its `aliases` and `extend-aliases`, and code
  reads such a module through its alias on purpose, so `np.array` stays.

  ```toml config
  [tool.simplint.imports]
  module-attribute-scope = ["third-party"]
  ```

  ```python ok
  import numpy as np

  values = np.array([1, 2])
  ```

- **A module Ruff bans importing names from**: a module its
  [`banned-from`](https://docs.astral.sh/ruff/settings/#lint_flake8-import-conventions_banned-from) lists, which
  [banned-import-from](https://docs.astral.sh/ruff/rules/banned-import-from/) (ICN003) reports `from module import name`
  for, stays reached through the module.

  ```toml config
  [tool.ruff.lint.flake8-import-conventions]
  banned-from = ["json"]

  [tool.simplint.imports]
  module-attribute-scope = ["stdlib"]
  ```

  ```python ok
  import json

  text = json.dumps([])
  ```

- **A dotted import of a module outside the run without an alias**: `import os.path` binds `os`, and its `path`
  attribute is the imported submodule rather than a name to import.

  ```toml config
  [tool.simplint.imports]
  module-attribute-scope = ["stdlib"]
  ```

  ```python ok
  import os.path

  joined = os.path.join("a", "b")
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

- **A module used as a value**: passed to a function, assigned or returned, as in
  [`getattr(findings, "DbWriter")`](https://docs.python.org/3/builtins/functions.html#getattr) or
  [`importlib.reload(findings)`](https://docs.python.org/3/library/importlib.html#importlib.reload), the module is an
  object of its own, so the file keeps it.

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

- **Locals that shadow the module name**. An attribute read through a parameter or local variable named `findings`
  refers to that value. Other reads through the imported module still count.

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

- **Module dunders such as `__name__`**. Importing the attribute under its own name could replace the importing module's
  `__name__`.

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

  The same holds for a module outside the run that `module-attribute-scope` covers, such as
  `mocker.patch("requests.get")`.

  ```toml config
  [tool.simplint.imports]
  module-attribute-scope = ["requests"]
  ```

  ```python ok file=client.py
  import requests

  response = requests.get("https://example.com")
  ```

  ```python context file=tests/test_client.py
  def test_get(mocker):
      mocker.patch("requests.get")
  ```

- **An import with a comment inside its statement**, such as a comment after an opening parenthesis: the rule reports
  the accesses without a fix, because the fix would replace the import and delete the comment.

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/findings.py
  class DbWriter:
      pass
  ```

  ```python report file=pkg/consumer.py
  from . import (  # keep: the writer lives here
      findings,
  )

  writer = findings.DbWriter()
  ```

  ```python fixed file=pkg/consumer.py
  from . import (  # keep: the writer lives here
      findings,
  )

  writer = findings.DbWriter()
  ```

- **A module in an import cycle with the importing file**: the file and the module import each other at run time, so the
  from-import would read a name the module has not defined yet when the cycle loads it. The rule reports the accesses
  without a fix. Here `pkg/a.py` and `pkg/b.py` import each other, and each reads a name of the other. A back-import
  under `if TYPE_CHECKING:` is not a run-time import, so it forms no cycle.

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python report file=pkg/a.py
  from . import b

  VALUE = 1


  def run() -> int:
      return b.helper()
  ```

  ```python fixed file=pkg/a.py
  from . import b

  VALUE = 1


  def run() -> int:
      return b.helper()
  ```

  ```python report file=pkg/b.py
  from . import a


  def helper() -> int:
      return a.VALUE
  ```

  ```python fixed file=pkg/b.py
  from . import a


  def helper() -> int:
      return a.VALUE
  ```

### Fix

The fix is unsafe. After `from findings import DbWriter`, replacing `findings.DbWriter` during a test no longer changes
the name in the importing module.

simplint replaces a single module import and every attribute read through it together. The source module must define
each name, and the import statement must contain no comments. simplint withholds the fix if a computed write such as
[`setattr(module, name, value)`](https://docs.python.org/3/builtins/functions.html#setattr) could change an attribute,
if another file reads the imported module binding, if `__all__` exports it or if the modules share a runtime import
cycle. The first unsuppressed report carries the fix.

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

Here one fix imports both `DbWriter` and `Reader`.

```python context file=pkg/__init__.py
# A package.
```

```python context file=pkg/findings.py
class DbWriter:
    pass


class Reader:
    pass
```

```python report file=pkg/consumer.py
from . import findings

writer = findings.DbWriter()
reader = findings.Reader()
```

```python fixed file=pkg/consumer.py
from .findings import DbWriter, Reader

writer = DbWriter()
reader = Reader()
```

Reading `findings.__name__` still needs the module object, so `findings.DbWriter` gets a report without a fix.

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

Inside `make`, the bare name `Item` refers to a type parameter. Replacing `findings.Item()` with `Item()` would call
that parameter.

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

Importing `set` would shadow the builtin. The rule leaves `findings.set` qualified even if the file never calls the
builtin.

```python context file=pkg/__init__.py
# A package.
```

```python context file=pkg/findings.py
def set(key, value):
    return key, value
```

```python report file=pkg/consumer.py
from . import findings

configured = findings.set("a", 1)
values = set([1, 2])
```

```python fixed file=pkg/consumer.py
from . import findings

configured = findings.set("a", 1)
values = set([1, 2])
```

Replacing the import in `pkg.consumer` would break `from .consumer import findings` in `pkg/other.py`.

```python context file=pkg/__init__.py
# A package.
```

```python context file=pkg/findings.py
class DbWriter:
    pass
```

```python context file=pkg/other.py
from .consumer import findings
```

```python report file=pkg/consumer.py
from . import findings

writer = findings.DbWriter()
```

```python fixed file=pkg/consumer.py
from . import findings

writer = findings.DbWriter()
```

The import in `pkg/api.py` stays because `__all__` exports `findings`.

```python context file=pkg/__init__.py
# A package.
```

```python context file=pkg/findings.py
class DbWriter:
    pass
```

```python report file=pkg/api.py
from . import findings

__all__ = ["findings"]

writer = findings.DbWriter()
```

```python fixed file=pkg/api.py
from . import findings

__all__ = ["findings"]

writer = findings.DbWriter()
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

Only attribute reads on imported modules are in scope. Private names and private module paths receive no automatic fix
because the replacement import can violate [private-import-path](SMP310.html). Ruff's
[import-private-name](https://docs.astral.sh/ruff/rules/import-private-name/) (PLC2701) checks the import itself.
Selecting it leaves the attribute-access report enabled.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issues [#21025](https://github.com/astral-sh/ruff/issues/21025)
  and [#14070](https://github.com/astral-sh/ruff/issues/14070).
