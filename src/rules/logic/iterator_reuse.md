Flags repeated uses of a one-pass iterator, including uses inside a loop. A later read may see fewer items or none at
all.

## Why is this bad?

[`map()`](https://docs.python.org/3/builtins/functions.html#map),
[`filter()`](https://docs.python.org/3/builtins/functions.html#filter),
[`zip()`](https://docs.python.org/3/builtins/functions.html#zip), a generator expression and the other
[iterators](https://docs.python.org/3/glossary.html#term-iterator) hand out each item once. After `list(items)` or
[`sum(items)`](https://docs.python.org/3/builtins/functions.html#sum) finishes, the iterator is empty. Iterating over it
again produces no items.

## Example

```python report
def summary(numbers):
    texts = map(str, numbers)
    joined = ", ".join(texts)
    return joined, sorted(texts)
```

Use instead:

```python ok
def summary(numbers):
    texts = [str(number) for number in numbers]
    joined = ", ".join(texts)
    return joined, sorted(texts)
```

## What it does

### Active on

- **A second read of a name bound once to an iterator** in a function or at module level. The rule recognizes `map()`,
  `filter()`, `zip()`, [`reversed()`](https://docs.python.org/3/builtins/functions.html#reversed),
  [`enumerate()`](https://docs.python.org/3/builtins/functions.html#enumerate) or a generator expression. It counts
  reads by a `for` loop, a comprehension, `list()`, `tuple()`, `set()`, `frozenset()`,
  [`sorted()`](https://docs.python.org/3/builtins/functions.html#sorted), `sum()`,
  [`min()`](https://docs.python.org/3/builtins/functions.html#min),
  [`max()`](https://docs.python.org/3/builtins/functions.html#max), `dict()`, `.join()` on a string or bytes literal, or
  `*` unpacking. The rule also counts [`any()`](https://docs.python.org/3/builtins/functions.html#any) and
  [`all()`](https://docs.python.org/3/builtins/functions.html#all), which can stop partway through the iterator. It
  treats [`len()`](https://docs.python.org/3/builtins/functions.html#len) as a read too, although `len()` raises
  [`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError) for ordinary generators and `map` objects.
  Choose whether to keep the items in a list or build a fresh iterator for each use.

  ```python report
  def summary(numbers):
      texts = map(str, numbers)
      first = list(texts)
      return first, sorted(texts)
  ```

- **A read inside a loop** that does not bind the iterator itself: each pass starts where the previous one stopped.

  ```python report
  def pairs(rows, names):
      upper = (name.upper() for name in names)
      return [(row, list(upper)) for row in rows]
  ```

- **Parameters annotated `Iterator` or `Generator`**, from `typing` or
  [`collections.abc`](https://docs.python.org/3/library/collections.abc.html#module-collections.abc).

  ```python report
  from collections.abc import Iterator


  def average(lines: Iterator[str]) -> float:
      count = sum(1 for _ in lines)
      return sum(len(line) for line in lines) / count
  ```

### Exceptions

- **Separate branches.** The rule skips reads in different `if` or `match` branches or on opposite sides of a
  conditional expression. It also skips pairs in different `try` and `except` bodies, even though an exception can occur
  after a partial read. An earlier branch that ends with `return`, `raise`, `break` or `continue` can also exclude a
  later read.

  ```python ok
  def texts(numbers, ordered):
      found = map(str, numbers)
      if ordered:
          return sorted(found)
      return list(found)
  ```

- **A `for` loop that can `break`**: the next read resumes where the loop stopped, which is often the point.

  ```python ok
  def body(lines):
      rows = map(str.strip, lines)
      for row in rows:
          if not row:
              break
      return list(rows)
  ```

- **Any other read**, such as [`next(texts)`](https://docs.python.org/3/builtins/functions.html#next) or passing the
  iterator to a function: what it takes is not known.

  ```python ok
  def tail(numbers):
      texts = map(str, numbers)
      next(texts)
      return list(texts)
  ```

- **Parameters annotated `Iterable`**: a list is an `Iterable` too, so a second read may be fine.

  ```python ok
  from collections.abc import Iterable


  def average(values: Iterable[float]) -> float:
      return sum(values) / len(list(values))
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** [B031](https://docs.astral.sh/ruff/rules/reuse-of-groupby-generator/)
  reports the same mistake for the groups of
  [`itertools.groupby`](https://docs.python.org/3/library/itertools.html#itertools.groupby) only.
