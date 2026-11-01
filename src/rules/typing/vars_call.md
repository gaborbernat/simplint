## What it does

Checks for direct calls to the builtin [`vars()`](https://docs.python.org/3/builtins/functions.html#vars). Passing
`vars` as a value, or calling `builtins.vars(...)`, is not reported. Neither is a call through a parameter or local
variable named `vars`. Files that bind `vars` at module level, for example by defining a function of that name, are
skipped.

## Why is this bad?

`vars()` returns a `dict[str, Any]`, so the attribute types are lost and a misspelled key fails only at runtime. Reading
attributes from a typed object keeps both checks in the type checker. For
[`argparse`](https://docs.python.org/3/library/argparse.html), pass a
[`Namespace`](https://docs.python.org/3/library/argparse.html#argparse.Namespace) subclass with annotated attributes to
[`parse_args(namespace=...)`](https://docs.python.org/3/library/argparse.html#argparse.ArgumentParser.parse_args), as in
the example.

## Example

```python
def options(parser: argparse.ArgumentParser) -> dict[str, str]:
    return vars(parser.parse_args())
```

Use instead:

```python
class Options(argparse.Namespace):
    output: str


def options(parser: argparse.ArgumentParser) -> Options:
    return parser.parse_args(namespace=Options())
```

## Fix safety

No fix is offered: the typed replacement depends on how the result is used.
