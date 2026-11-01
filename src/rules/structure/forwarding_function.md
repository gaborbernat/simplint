## What it does

Checks for private functions and methods whose body, apart from a docstring, is a single call that is returned or used
as an expression statement. It covers module-level functions whose name starts with `_`, and methods with such names in
module-level classes without base classes. Methods are matched through `self.name`, `cls.name` or `ClassName.name`.

Every argument of the call must be one of the function's own parameters, passed as is, as `*args` or `**kwargs`
([arbitrary argument lists](https://docs.python.org/3/tutorial/controlflow.html#arbitrary-argument-lists)), or as
`keyword=parameter`. The call target must be a name or a dotted name such as `self.client.get`.

These are skipped:

- [dunder](https://docs.python.org/3/reference/datamodel.html#special-method-names) and decorated functions
- functions the file never refers to, or refers to anywhere without calling, for example when passing one as a callback
- wrappers that add their own values, such as `check=True`, a computed argument or a parameter default, since they
  configure the call
- functions another file in the run patches by name, such as with `mocker.patch("pkg.module._load")`, since tests
  replace them
- calls on a computed value, such as `make(name).strip()`
- calls whose target starts with one of the function's own parameters, such as `fn` in `def _apply(fn, x): return fn(x)`

## Why is this bad?

The wrapper adds a name and an extra hop without adding behavior, and a reader has to open it to learn that it calls
something else. Calling the target directly removes the indirection.

## Example

```python
def config_path() -> Path:
    return _make_path("config.toml").resolve()


def _make_path(name: str) -> Path:
    return Path(name)
```

Use instead:

```python
def config_path() -> Path:
    return Path("config.toml").resolve()
```

## Fix safety

No fix is offered, since inlining needs each call site's arguments mapped onto the target's parameters.
