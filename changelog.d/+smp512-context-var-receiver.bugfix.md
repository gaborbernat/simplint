[`up-context-var-token-reset`](../src/rules/logic/up_context_var_token_reset.md) only reports a `set`/`reset` pair on a
name assigned a `contextvars.ContextVar` or a parameter annotated as one, so another class's `set` is no longer
rewritten into a `with` block that raises `TypeError`.
