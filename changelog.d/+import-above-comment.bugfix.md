[`up-chdir-restore`](../src/rules/logic/up_chdir_restore.md),
[`up-asyncio-iscoroutinefunction`](../src/rules/logic/up_asyncio_iscoroutinefunction.md) and
[`missing-final`](../src/rules/typing/missing_final.md) put the import they add above the comment lines that describe
the statement it precedes, instead of between that comment and the statement. A line of a docstring that starts with `#`
is not such a comment, so the import stays out of the docstring.
