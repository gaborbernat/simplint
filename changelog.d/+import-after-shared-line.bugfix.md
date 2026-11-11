[`up-asyncio-iscoroutinefunction`](../src/rules/logic/up_asyncio_iscoroutinefunction.md),
[`up-chdir-restore`](../src/rules/logic/up_chdir_restore.md) and [`missing-final`](../src/rules/typing/missing_final.md)
keep their report without a fix when the last import shares its line with code, since the new import used to land below
that code, which then ran before it.
