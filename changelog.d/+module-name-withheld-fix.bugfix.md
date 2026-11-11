[`up-context-var-token-reset`](../src/rules/logic/up_context_var_token_reset.md) and
[`up-chdir-restore`](../src/rules/logic/up_chdir_restore.md) keep their report without a fix when another module reads
the module-level name they would remove, through an import, a star import or an attribute, or when `__all__` lists it,
so the fix no longer breaks that module.
