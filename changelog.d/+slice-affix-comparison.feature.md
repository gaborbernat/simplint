[`slice-affix-comparison`](../src/rules/logic/slice_affix_comparison.md) reports `s[:n] == "..."` and `s[-n:] == "..."`
with a literal of length `n`. When `s` is surely a `str` or `bytes`, its unsafe fix calls `startswith()` or `endswith()`
instead.
