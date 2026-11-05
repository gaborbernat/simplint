`up-context-var-token-reset` (SMP512) flags a `ContextVar.set()` token that a `try` block resets in its `finally`
clause, which a `with` block does on Python 3.14 and later, with a safe fix.
