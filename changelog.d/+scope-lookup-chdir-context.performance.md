[`up-chdir-restore`](../src/rules/logic/up_chdir_restore.md) and
[`up-context-var-token-reset`](../src/rules/logic/up_context_var_token_reset.md) find the function or class around a
statement with a search over the file's scopes instead of a scan of all of them. A module of 32,000 functions with a
`finally` block each takes 0.22 seconds under SMP512 instead of 1.2, and 5.2 seconds under SMP513 instead of 7.6.
