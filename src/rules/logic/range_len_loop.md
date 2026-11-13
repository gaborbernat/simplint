Flags `for i in range(len(seq))` loops that read `seq[i]`. Looping over `seq` gives each item directly, and
[`enumerate()`](https://docs.python.org/3/library/functions.html#enumerate) gives the position too when the loop needs
it.

## Why is this bad?

The index loop spells out what `for` already does: it looks each item up again by position, and every `seq[i]` is a
place to pick the wrong sequence or index. It also works only for sequences, while `for item in seq` takes any iterable.

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

- **`for` loops over `range(len(seq))`**, where `seq` is a name or an attribute chain, whose body reads `seq[i]`.
  **Unsafe fix:** the loop iterates `seq` and binds the singular of its name, `names` giving `name`, which replaces each
  `seq[i]`. It is offered when `i` is read only as `seq[i]` and nowhere after the loop, the body does not rebind, assign
  to or call a method of `seq`, and the singular is a name the loop's function, or the module for a module-level loop,
  does not use yet. It is unsafe because a class may define `__getitem__` and `__len__` with an `__iter__` that yields
  something else, as a mapping does.

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

- [Pylint](https://github.com/pylint-dev/pylint) reports the same loops as
  [consider-using-enumerate](https://pylint.readthedocs.io/en/stable/user_guide/messages/convention/consider-using-enumerate.html)
  (C0200), with no fix.
