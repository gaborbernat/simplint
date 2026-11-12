Flags a one-pass iterator read a second time, or read inside a loop, where every read after the first sees it empty.

## Why is this bad?

[`map()`](https://docs.python.org/3/library/functions.html#map),
[`filter()`](https://docs.python.org/3/library/functions.html#filter),
[`zip()`](https://docs.python.org/3/library/functions.html#zip), a generator expression and the other
[iterators](https://docs.python.org/3/glossary.html#term-iterator) hand out each item once. The first `list()`, `for`
loop or `sum()` takes every item, and the next read gets nothing, without an error. Code that worked while the value was
a list breaks quietly when it becomes a `map()`.

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

- **A second read of a local bound once to an iterator**: `map()`, `filter()`, `zip()`, `reversed()`, `enumerate()` or a
  generator expression, read to its end by a `for` loop, a comprehension, `list()`, `tuple()`, `set()`, `frozenset()`,
  `sorted()`, `sum()`, `min()`, `max()`, `any()`, `all()`, `dict()`, `str.join()` or `*` unpacking. The read that finds
  the iterator empty is reported. No fix: keeping a list and building the iterator again are both valid.

  ```python report
  def summary(numbers):
      texts = map(str, numbers)
      first = list(texts)
      return first, sorted(texts)
  ```

- **A read inside a loop** that does not bind the iterator itself: the first pass empties it.

  ```python report
  def pairs(rows, names):
      upper = (name.upper() for name in names)
      return [(row, list(upper)) for row in rows]
  ```

- **Parameters annotated `Iterator` or `Generator`**, from `typing` or `collections.abc`.

  ```python report
  from collections.abc import Iterator


  def average(lines: Iterator[str]) -> float:
      count = sum(1 for _ in lines)
      return sum(len(line) for line in lines) / count
  ```

### Exceptions

- **Reads that never both run**, in different arms of an `if`, `match` or `try`, or after an arm that returns, raises,
  breaks or continues.

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

- **Any other read**, such as `next(texts)` or passing the iterator to a function: what it takes is not known.

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
  reports the same mistake for the groups of `itertools.groupby` only.
