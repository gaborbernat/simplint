[`up-chdir-restore`](../src/rules/logic/up_chdir_restore.md),
[`up-asyncio-iscoroutinefunction`](../src/rules/logic/up_asyncio_iscoroutinefunction.md),
[`up-codecs-open`](../src/rules/logic/up_codecs_open.md) and
[`up-removed-ast-node`](../src/rules/logic/up_removed_ast_node.md) keep their report without a fix when a parameter or
local, a loop variable or a lambda parameter of a scope that encloses the call takes the module or builtin name the
rewrite uses, so the rewritten code no longer calls that binding. A nested function or a comprehension elsewhere in the
function does not count.
