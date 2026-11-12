Flags own-line comments whose every word is an identifier or keyword of the statement below, such as `# close file`
above [`file.close()`](https://docs.python.org/3/reference/datamodel.html#file.close). A statement across several lines
counts in full.

## Why is this bad?

A comment that repeats the line under it tells the reader nothing new, and it goes stale without anyone noticing when
the code changes. Comments are for the reasons behind the code, which the code cannot show.

## Example

```python report
# close file
file.close()
```

Use instead:

```python ok
file.close()
```

## What it does

### Active on

- **Own-line comments above code**: every word of the comment appears among the identifiers and keywords of the next
  line. Words compare without case, and a name matches only as a whole: `# create user` does not restate
  `create_user()`, since `create` and `user` are not words of the code.

  Consecutive own-line comments form one block, which is judged as a whole and removed whole.

  ```python report
  def run(result: int) -> int:
      # return result
      return result
  ```

  ```python report
  # Create user
  user.create()
  ```

### Exceptions

- **Comments with a word the code lacks**: the comment says something the code does not, so it stays.

  ```python ok
  # create the user for the request
  user = create_user()
  ```

- **Comments with a colon**: a colon marks a label or a directive, such as `# Note: ` or `# type: ignore`.

  ```python ok
  # Note: create user
  user = create_user()
  ```

- **Comments that quote syntax**: punctuation other than a sentence's, such as `*`, `[`, `(`, `=` or `>`, writes a piece
  of syntax or a condition, which says more than the names in it.

  ```python ok
  # *args
  args.extend(extra)
  ```

  ```python ok
  # depth > limit
  depth = limit + 10
  ```

- **Trailing comments**: a comment after code describes that line, not the line below.

  ```python ok
  user = create_user()  # create user
  ```

### Fix

The fix deletes the comment's line. It is safe, since a comment never changes behavior, and the words it had are all in
the code it leaves in place.

```python report
# close file
file.close()
```

```python fixed
file.close()
```

## Ruff overlap

Ruff's [commented-out-code](https://docs.astral.sh/ruff/rules/commented-out-code/) (ERA001) reports some of the same
comments without a fix that applies. This rule keeps those comments and reports them with its safe fix, which deletes
the comment.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [ERA001](https://docs.astral.sh/ruff/rules/commented-out-code/), which reports some of the same comments without an
  applicable fix; this rule keeps them (see Ruff overlap).
- [flake8-comments](https://github.com/orsinium-labs/flake8-comments) reports the same comments as
  [CM001](https://github.com/orsinium-labs/flake8-comments/blob/master/flake8_comments/_checker.py), such as
  `# create user` above `user.create()`. It only reports them, where this rule's fix deletes the comment.
