Flags `for i in range(len(seq))` loops that read `seq[i]`. Looping over `seq` gives each item directly, and
[`enumerate()`](https://docs.python.org/3/builtins/functions.html#enumerate) gives the position too when the loop needs
it.

## Why is this bad?

When a loop only uses the index to read each item, iterating over the items removes a lookup and works with iterables
that have no length or integer indexes.

## Example

```python report
def show(names):
    for i in range(len(names)):
        print(names[i])
```

Use instead:

```python ok
def show(names):
    for name in names:
        print(name)
```

## What it does

### Active on

- **`for` loops over `range(len(seq))`**, where `seq` is a name or an attribute chain, whose body reads `seq[i]`. The
  unsafe fix replaces `names[i]` with `name` and iterates over `names`. The loop must be the only binding of `i`, and
  every read of `i` must index `seq` in the loop body. The fix skips bodies that rebind the sequence's root name, access
  an attribute of `seq`, or write or otherwise subscript `seq`. To name the item it strips a final `s` and any leading
  underscores. It skips candidates that still end in `s`, conflict with a keyword or built-in, or already occur in the
  scope. Check what iterating over the object yields. For example, a mapping yields its keys even if it also supports
  integer indexing.

  ```python report
  def show(names):
      for i in range(len(names)):
          print(names[i])
  ```

  ```python fixed
  def show(names):
      for name in names:
          print(name)
  ```

- **Loops that also need the index**, reported without a fix: `enumerate()` gives both.

  ```python report
  def show(names):
      for i in range(len(names)):
          print(i, names[i])
  ```

- **Loops that assign to `seq[i]`**, reported without a fix: the loop could build a new list instead.

  ```python report
  def strip_all(lines):
      for i in range(len(lines)):
          lines[i] = lines[i].strip()
  ```

### Exceptions

- **Loops that never read `seq[i]`**: the index is the point, or it indexes another sequence.

  ```python ok
  def pair(names, labels):
      for i in range(len(names)):
          print(labels[i])
  ```

- **`range(len(...))` of a call or another expression**, which the loop could not read again unchanged.

  ```python ok
  def show(load):
      for i in range(len(load())):
          print(load()[i])
  ```

## Other tools

- [Pylint](https://github.com/pylint-dev/pylint) suggests `enumerate()` for index loops in
  [consider-using-enumerate](https://pylint.readthedocs.io/en/stable/user_guide/messages/convention/consider-using-enumerate.html)
  (C0200), with no fix.
