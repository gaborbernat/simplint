Flags requirements in `[project] dependencies` that no file of the run outside the tests imports.

## Why is this bad?

Every requirement is installed for every user of the project. One the code no longer imports costs them download time
and disk space, and its version bounds can still conflict with what else they install. A requirement only the tests
import belongs in a dependency group, where the tests' installs find it and users' installs skip it.

## Example

```toml config
[project]
name = "app"
dependencies = ["requests", "pyyaml"]
```

```python report file=app/__init__.py
import requests
```

Use instead:

```toml config
[project]
name = "app"
dependencies = ["requests"]
```

```python ok file=app/__init__.py
import requests
```

## What it does

The rule collects every absolute import in the run's files outside the tests, in any scope, deferred and guarded ones
included, and matches them to the requirements the way [`undeclared-dependency`](SMP312.html) does. The project has no
Python file for `pyproject.toml`, so the reports sit at the start of the `__init__.py` of its first top-level package.

A requirement the code reaches without an import is reported too, such as a tool it runs as a command or a module a
compiled extension loads. A suppression comment there says why it stays.

### Exceptions

- **Requirements with an environment marker**, such as `tomli; python_version < "3.11"`: they install only where the
  marker holds, which an import behind a version check matches.
- **Projects whose `[project]` leaves `dependencies` to the build backend** through `dynamic`, or that have no
  `[project]` table.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** no rule.
- [deptry](https://github.com/fpgmaas/deptry) reports the same requirements as DEP002, in `pyproject.toml` itself, with
  its own name mapping read from the installed distributions.
