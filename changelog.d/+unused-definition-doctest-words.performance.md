[`unused-definition`](../src/rules/api/unused_definition.md) reads the words of a file's doctests once instead of
rescanning every docstring for each definition it checks, so 16,000 documented functions take 0.06 seconds instead of
3.1.
