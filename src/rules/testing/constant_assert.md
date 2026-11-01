## What it does

Checks for [`assert`](https://docs.python.org/3/reference/simple_stmts.html#the-assert-statement) statements whose test
is a [truthy](https://docs.python.org/3/builtins/stdtypes.html#truth-value-testing) constant, judged the way Ruff judges
truthiness: `True`, a non-zero number, a non-empty string, bytes or f-string literal, `...`, a lambda, or a non-empty
tuple, list, set or dict literal such as `[value]`. A container holding only starred elements, such as `[*values]`, may
be empty and is not reported. Falsy constants such as `assert False`, `assert 0` or `assert ()` always fail, often on
purpose, and are not reported either. The rule checks every file, not only tests.

## Why is this bad?

An assertion on a truthy constant can never fail, so it checks nothing and makes a test look like it verifies more than
it does. A parenthesized `assert (check, "message")` is a non-empty tuple and falls in the same trap; Ruff reports that
case as [F631](https://docs.astral.sh/ruff/rules/assert-tuple/).

## Example

```python
def test_save(tmp_path: Path) -> None:
    save(tmp_path / "out.json")
    assert True
```

Use instead:

```python
def test_save(tmp_path: Path) -> None:
    save(tmp_path / "out.json")
    assert (tmp_path / "out.json").exists()
```

## Fix safety

A two-element tuple without a message, such as `assert (check, "message")`, becomes `assert check, "message"`. That fix
is unsafe, since the restored check may now fail. Other tuples get no fix.

For any other constant the fix is safe: it deletes the assertion, which never fails, and leaves `pass` when the
assertion was the only statement in its block. No fix is offered when the assertion shares a line with another
statement, or when its test calls something, as in `assert [compute()]`, since deleting it would skip that call.
