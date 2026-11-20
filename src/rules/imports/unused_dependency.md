Flags unconditional `project.dependencies` entries with no matching import outside the tests.

## Why is this bad?

Users install unconditional requirements even when the project never uses them. An unnecessary dependency costs download
time and disk space, and its version bounds can conflict with other packages. Move test-only requirements to a
dependency group for tests.

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
included, and matches them to the requirements the way [`undeclared-dependency`](SMP312.html) does. Reports appear at
the start of `__init__.py` in the first configured top-level package. Without that file in the run, there is nowhere to
report the requirement.

Review each finding before removing the requirement. Distribution names can differ from import names, and a project
might invoke a dependency as a command or load it from a compiled extension. A suppression at the report can record why
the requirement stays.

### Exceptions

- **Requirements with an environment marker**, such as `tomli; python_version < "3.11"`: the rule skips these even if it
  finds no matching import.
- **Projects without `project.name` or with `dependencies` listed in `project.dynamic`**. The rule also skips nested
  projects and dependencies that name the project itself.

## Other tools

[deptry's DEP002](https://deptry.com/rules-violations/#unused-dependencies-dep002) reports unused dependencies in the
dependency declaration. deptry uses the installed distributions to help map package names to import names.
