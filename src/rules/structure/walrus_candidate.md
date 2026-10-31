## What it does

Checks for an assignment to a single name that is directly followed by an `if` statement whose test starts with that
name. The test starts with the name when the name is:

- the whole test, as in `if found:`
- the operand of `not`, as in `if not found:`
- the left side of a comparison, as in `if found > 3:`
- the first operand of `and` or `or`, as in `if found and ready:`

Nothing else may read the name: not after the `if` statement, and not before the assignment either, such as a closure or
an earlier statement of the same loop. The search covers the enclosing function, or the whole file at module level.

The rule does not run when the target Python version is older than 3.8, which introduced `:=`. It skips:

- assignments in a class body, including inside an `if`, `for` or `try` block of the class, unless a function sits
  between
- values that are a lambda, `yield`, `yield from`, an `:=` expression or a tuple without parentheses
- values that mention the name itself
- assignments that share a line with other code
- assignments with a comment on their line, or between them and the `if`
- `TYPE_CHECKING`, since type checkers recognize the guard by that name
- module-level names that another module in the run imports or patches by name, or that `__all__` lists, and every
  module-level name when `__all__` is computed
- names the function declares [`global`](https://docs.python.org/3/reference/simple_stmts.html#the-global-statement) or
  [`nonlocal`](https://docs.python.org/3/reference/simple_stmts.html#the-nonlocal-statement)

## Why is this bad?

A name that exists only to feed the next test splits one decision over two statements. It also stays in scope after it
stops mattering. Inlining the value, or binding it in the test with an
[assignment expression](https://docs.python.org/3/reference/expressions.html#assignment-expressions) (`:=`, from
[PEP 572](https://peps.python.org/pep-0572/)), keeps the value next to the check that uses it.

## Example

```python
def first_word(text: str) -> str:
    found = re.match(r"\w+", text)
    if found:
        return found.group(0)
    stripped = text.strip()
    if not stripped:
        return ""
    return text
```

Use instead:

```python
def first_word(text: str) -> str:
    if found := re.match(r"\w+", text):
        return found.group(0)
    if not text.strip():
        return ""
    return text
```

## Fix safety

The fix is safe. It deletes the assignment. When the name appears nowhere else in the `if` statement, including the rest
of the test, the body and any `elif` or `else` branch, the fix inlines the value and adds parentheses where precedence
requires them. Otherwise it writes `name := value` in the test, in parentheses unless the name is the whole test.

The search for other reads looks only at names in code. It misses importers outside the run and references held in
strings, such as an `__all__` entry.

## Options

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python)
