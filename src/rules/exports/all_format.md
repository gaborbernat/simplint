## What it does

Checks module-level `=`, annotated and augmented assignments to
[`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package) whose value is a list or tuple
literal:

- A list must have a trailing comma after its last name. An empty list is accepted, since it has no name to put a comma
  after. With Ruff's
  [`skip-magic-trailing-comma`](https://docs.astral.sh/ruff/settings/#format_skip-magic-trailing-comma) set, the
  formatter drops that comma, so only the list is required.
- A tuple is always reported, whether parenthesized or bare, such as `__all__ = "run", "stop"`.

Other values, such as `sorted([...])`, are not checked.

## Why is this bad?

A trailing comma after the last name makes Ruff's formatter put each name on its own line, unless
[`skip-magic-trailing-comma`](https://docs.astral.sh/ruff/settings/#format_skip-magic-trailing-comma) is set. Adding or
removing an export then changes one line, so the diff shows exactly which name changed. Using a list in every module
gives `__all__` one shape to read and edit, and it is the shape the [SMP113](SMP113.html) fix writes for a new
`__all__`. To also sort the names, see Ruff's [RUF022](https://docs.astral.sh/ruff/rules/unsorted-dunder-all/).

## Example

```python
__all__ = ("load", "save")
```

Use instead:

```python
__all__ = [
    "load",
    "save",
]
```

## Fix safety

Adding the missing comma to a list is safe. Rewriting a tuple as a list is unsafe, because code that concatenates
`__all__` with another tuple stops working. An empty tuple becomes `[]`.

The fix keeps the names in their order. A literal written on one line becomes a list with one name per line and a
trailing comma, the layout `ruff format` gives it, so `("load", "save")` needs no formatter pass. A literal that already
spans several lines keeps its layout: the fix swaps the brackets and adds the comma.

## Options

- [`format.skip-magic-trailing-comma`](https://docs.astral.sh/ruff/settings/#format_skip-magic-trailing-comma)
