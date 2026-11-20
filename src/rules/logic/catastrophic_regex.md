Flags a regular expression with a nested repeat that can take exponential time on an input that almost matches, such as
`(a+)+$`.

## Why is this bad?

Python's `re` engine can try many ways of splitting the same text between an inner and outer repeat. If a later part of
the pattern fails, the number of splits it tries can grow exponentially with the input length. This is a form of
[regular expression denial of service (ReDoS)](https://owasp.org/www-community/attacks/Regular_expression_Denial_of_Service_-_ReDoS).

## Example

```python report
import re

WORDS = re.compile(r"^(\w+\s?)*$")
```

For validation that uses only the success or failure of the match, make whitespace the required separator between words.
The optional outer group preserves the empty match.

```python ok
import re

WORDS = re.compile(r"^(?:\w+(?:\s\w+)*\s?)?$")
```

## What it does

### Active on

- **Patterns passed to `re` and `regex` functions**, as string or bytes literals, verbose ones included. The rule
  reports an unbounded repeat, `*`, `+` or `{n,}`, lazy or greedy, of a group where one alternative contains an
  unbounded repeat and all the other parts of that alternative are optional. Examples include `(a+)+`, `(\w+\s?)*` and
  `(x|y+)+`.

### Exceptions

- **Parts inside the group that cannot match the same text**, as `(\d+\.)+` is, where each repeat must end with a `.`.

- **Possessive quantifiers and atomic groups**, which Python 3.11 added and which never backtrack into what they
  matched, as `(a+)++` and `(?>a+)+`.

  ```python ok
  import re

  WORDS = re.compile(r"^(?>\w+\s?)*$")
  ```

- **A repeat whose remaining pattern always succeeds.** In a prefix match or search, the engine can stop at the first
  match. A trailing anchor or lookaround can still fail. The rule also checks repeats at the end of patterns passed to
  `fullmatch` or `compile`, since those patterns may have to consume the whole string.

  ```python ok
  import re


  def strip_suffix(version: str) -> str:
      return re.sub(r"[.]a([.]?([0-9]+))*", "", version)
  ```

- **Bounded repeats**, as `(a+){3}`, whose backtracking stays polynomial.

- **Patterns the rule does not read**, with a conditional group `(?(1)...)` or a scoped verbose flag `(?x:...)`.

## Other tools

- **[dlint](https://github.com/dlint-py/dlint/blob/master/docs/linters/DUO138.md):** DUO138 reports nested quantifiers
  in `re` patterns.
