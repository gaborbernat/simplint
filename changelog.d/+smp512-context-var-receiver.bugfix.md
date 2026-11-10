`up-context-var-token-reset` only reports a `set`/`reset` pair on a name assigned a `contextvars.ContextVar` or a
parameter annotated as one, so another class's `set` is no longer rewritten into a `with` block that raises `TypeError`.
