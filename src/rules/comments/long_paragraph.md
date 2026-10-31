## What it does

Checks for comment and docstring paragraphs longer than 100 words.

- In comments, a paragraph is a run of own-line comments on consecutive lines at the same indentation. An empty `#`
  line, a line of code or a comment at another indentation ends it. Trailing comments are ignored. The report spans from
  the paragraph's first line to the line where it passes 100 words.
- In [docstrings](https://peps.python.org/pep-0257/) of modules, classes and functions, an empty or whitespace-only line
  separates paragraphs. The report covers the whole docstring.

## Why is this bad?

A paragraph that long usually covers more than one topic, and readers skim it. Split at a change of topic, each part is
easier to find and to keep accurate.

## Example

```python
# Manifests are cached by path and modification time, because parsing them dominates the start-up time of large
# projects. ... The cache is dropped when the interpreter version changes, since the parsed form depends on it.
# (110 words in total)
```

Use instead:

```python
# Manifests are cached by path and modification time, because parsing them dominates the start-up time of large
# projects. ...
#
# The cache is dropped when the interpreter version changes, since the parsed form depends on it. ...
```

## Fix safety

No fix is offered, since where to split a paragraph depends on its meaning.
