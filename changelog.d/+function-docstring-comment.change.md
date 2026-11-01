`function-docstring` (SMP002) keeps the text of a docstring that may state a contract: its fix turns any docstring
longer than one line of five words into a `#` comment block in its place, wrapped to Ruff's line and doc line limits,
instead of deleting it. A one-line docstring of at most five words, such as `"""Return the value."""`, is still deleted.
