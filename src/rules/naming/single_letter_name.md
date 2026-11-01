## What it does

Checks for single-letter names. It reports the names of functions, classes, parameters (including
[lambda](https://docs.python.org/3/reference/expressions.html#lambda) parameters), assignment and
[walrus](https://docs.python.org/3/reference/expressions.html#assignment-expressions) targets, `with ... as`,
`except ... as` and `import ... as` names,
[`match`](https://docs.python.org/3/reference/compound_stmts.html#the-match-statement) captures and
[type parameters](https://docs.python.org/3/reference/compound_stmts.html#type-params) such as `T`.

These pass:

- `for` loop targets, such as `i` in `for i in range(3)`
- every name inside a list, set or dictionary
  [comprehension](https://docs.python.org/3/tutorial/datastructures.html#list-comprehensions) or a
  [generator expression](https://docs.python.org/3/reference/expressions.html#generator-expressions), including a walrus
  target or lambda parameter, so `[x * 2 for x in values]` is fine
- `_`
- parameters of a method decorated with [`@override`](https://docs.python.org/3/library/typing.html#typing.override),
  which must keep the names of the method it replaces

Each name is reported once per scope, at its first binding. Ruff's
[ambiguous-variable-name (E741)](https://docs.astral.sh/ruff/rules/ambiguous-variable-name/) covers only `l`, `O` and
`I`.

## Why is this bad?

A single letter says nothing about what the name holds, so the reader has to find where it is bound to learn it. Loop
targets and comprehensions are exempt because the iterable the names come from sits right next to them.

## Example

```python
def scale(v: float, f: float) -> float:
    r = v * f
    return round(r, 2)
```

Use instead:

```python
def scale(value: float, factor: float) -> float:
    scaled = value * factor
    return round(scaled, 2)
```

## Fix safety

No fix is offered, since a meaningful name cannot be derived from the code.
