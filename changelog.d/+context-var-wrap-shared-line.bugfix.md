[`up-context-var-token-reset`](../src/rules/logic/up_context_var_token_reset.md) reports without a fix when the
assignment shares its line with an earlier statement. The fix used to leave that statement on the `with` line, which
does not parse.
