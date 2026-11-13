Flags a regular expression with a nested repeat that can take exponential time on an input that almost matches, such as
`(a+)+$`.

## Why is this bad?

Python's `re` engine backtracks. When a repeated group holds a repeat that can match the same text, as `(a+)+` does, a
run of `a`s splits between the outer and the inner repeat in exponentially many ways. On a string that fails to match
near its end, the engine tries every split before giving up: `(a+)+$` takes about a second on 24 `a`s followed by a `b`,
and doubles with each further `a`. A pattern that runs on user input this way lets a short request hang a worker, a
denial of service known as
[ReDoS](https://owasp.org/www-community/attacks/Regular_expression_Denial_of_Service_-_ReDoS).

## Example

```python report
import re

WORDS = re.compile(r"^(\w+\s?)*$")
```

Use instead:

```python ok
import re

WORDS = re.compile(r"^\w+(?:\s\w+)*\s?$")
```

## What it does

### Active on

- **Patterns passed to `re` and `regex` functions**, as string or bytes literals, verbose ones included. The rule
  reports an unbounded repeat, `*`, `+` or `{n,}`, lazy or greedy, of a group where one alternative is a single
  unbounded repeat with everything else in it optional, as `(a+)+`, `(\w+\s?)*` and `(x|y+)+` are.

### Exceptions

- **Parts inside the group that cannot match the same text**, as `(\d+\.)+` is, where each repeat must end with a `.`.

- **Possessive quantifiers and atomic groups**, which Python 3.11 added and which never backtrack into what they
  matched, as `(a+)++` and `(?>a+)+`.

  ```python ok
  import re

  WORDS = re.compile(r"^(?>\w+\s?)*$")
  ```

- **A repeat followed only by parts that can match nothing**, in a call other than `fullmatch` or `compile`: the first
  way the repeat matches lets the rest match too, so nothing makes the engine backtrack into it. An anchor such as `$`
  or a lookaround after the repeat can fail, and `fullmatch` or a compiled pattern may have to reach the end, so those
  stay reported.

  ```python ok
  import re


  def strip_suffix(version: str) -> str:
      return re.sub(r"[.]a([.]?([0-9]+))*", "", version)
  ```

- **Bounded repeats**, as `(a+){3}`, whose backtracking stays polynomial.

- **Patterns the rule does not read**, with a conditional group `(?(1)...)` or a scoped verbose flag `(?x:...)`.

## Ruff overlap

Ruff has no rule for exponential backtracking.

## Other tools

- **[dlint](https://github.com/dlint-py/dlint/blob/master/docs/linters/DUO138.md):** DUO138 reports nested quantifiers
  in `re` patterns.
