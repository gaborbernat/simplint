## What it does

Checks for names in [`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package) that no other
module of the run imports. It is the counterpart of [SMP113](SMP113.html), which adds the names other modules import:
together they keep `__all__` to the names the rest of the project uses. These count as imports, as for SMP113:
`from module import name`, `module.name` after `import module`, the entry points `pyproject.toml` declares, and the
declarations of the module's `.pyi` stub.

These modules are not checked:

- the top-level package or module of a library, and the modules the `api-modules` setting names, whose `__all__`
  declares the interface users outside the project import
- modules another file star-imports, since `import *` hands every listed name to the importing module
- test modules, as defined in [SMP200](SMP200.html)
- modules that build `__all__` dynamically, such as with `list(...)`

Once a name leaves `__all__`, [SMP103](SMP103.html) asks for a leading underscore, so what looks public is what other
modules use.

## Why is this bad?

`__all__` tells readers, and tools such as type checkers and documentation generators, which names a module offers. A
name nothing imports is not part of any interface; listing it makes the module look larger than it is, and keeps a stale
entry after its last user is gone.

## Example

```python
def parse(text: str) -> Tree: ...


def tokenize(text: str) -> list[str]: ...


__all__ = ["parse", "tokenize"]  # only `parse` is imported elsewhere
```

Use instead:

```python
def parse(text: str) -> Tree: ...


def _tokenize(text: str) -> list[str]: ...


__all__ = ["parse"]
```

## Fix safety

The fix removes the names from `__all__`: their lines when each name has a line of its own, or the names from a list
written on one line. In a library it is unsafe, since code outside the run may import the name; in an application, which
nothing outside the run imports, it is safe.

No fix is offered when the list holds a comment and its names share lines, or when more than one statement builds
`__all__`, such as `__all__ += [...]`.

## Options

- `project-kind` in `[tool.simplint]`: whether the project is a library or an application
- `api-modules` in `[tool.simplint]`: modules whose `__all__` stays as written
