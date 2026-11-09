SMP407 allows test package initializers, PEP 562 hooks and `TYPE_CHECKING` blocks. SMP530 compares duplicate code within
distributions, and SMP528 respects `@override` return contracts. SMP202 and SMP505 account for enabled Ruff limits
before suggesting changes; SMP603 checks whether a test calls its callable parameter. SMP401 leaves `__main__.py` alone,
and SMP809 allows `create_autospec` imports without patching helpers.
