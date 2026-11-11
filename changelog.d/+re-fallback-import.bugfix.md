[`up-re-error-alias`](../src/rules/logic/up_re_error_alias.md) and
[`up-re-prefixmatch`](../src/rules/logic/up_re_prefixmatch.md) leave a `re` alone when another import of that name, such
as `import regex as re` in a `try` block, binds a different module, so the fix no longer renames an `except` clause to a
name the other module lacks.
