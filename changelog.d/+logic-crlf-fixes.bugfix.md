The fixes of [`up-context-var-token-reset`](../src/rules/logic/up_context_var_token_reset.md),
[`up-chdir-restore`](../src/rules/logic/up_chdir_restore.md),
[`up-warnings-deprecated`](../src/rules/logic/up_warnings_deprecated.md) and
[`early-assignment`](../src/rules/logic/early_assignment.md) write the file's own line ending, so a CRLF file no longer
gets LF lines mixed in.
