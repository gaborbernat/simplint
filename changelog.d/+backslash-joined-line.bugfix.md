[`up-context-var-token-reset`](../src/rules/logic/up_context_var_token_reset.md) and
[`up-chdir-restore`](../src/rules/logic/up_chdir_restore.md) report without a fix when a backslash joins the assignment
to the line before it, instead of producing a syntax error that reverts the file.
