Flags comments on their own lines when every word repeats an identifier or keyword in the next statement. A multiline
statement counts in full.

## Why is this bad?

`# close file` adds nothing above [`file.close()`](https://docs.python.org/3/reference/datamodel.html#file.close). A
comment explaining why the file must close at that point gives the reader information the call itself omits.
[PEP 8](https://peps.python.org/pep-0008/#inline-comments) and the
[Google Python Style Guide](https://google.github.io/styleguide/pyguide.html#s3.8.5-block-and-inline-comments) both
discourage comments that repeat the code.

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
  statement. Names match as whole words, ignoring case. `# create user` does not restate `create_user()`, since `create`
  and `user` are not words of the code.

  Consecutive own-line comments form one block, so a word on any line can keep the whole block.

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

- **Comments that quote syntax**. Characters such as `*`, `[`, `(`, `=` or `>` can express a condition or quote code.
  The rule skips comments containing them.

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

The safe fix removes the matching comment block.

```python report
# close file
file.close()
```

```python fixed
file.close()
```

## Ruff overlap

Ruff's [commented-out-code](https://docs.astral.sh/ruff/rules/commented-out-code/) (ERA001) reports some of the same
comments. simplint offers a safe fix for those it identifies as restatements.

## Other tools

- [flake8-comments](https://github.com/orsinium-labs/flake8-comments) reports redundant comments as
  [CM001](https://github.com/orsinium-labs/flake8-comments/blob/master/flake8_comments/_checker.py), such as
  `# create user` above `user.create()`. simplint can also remove the comment.
