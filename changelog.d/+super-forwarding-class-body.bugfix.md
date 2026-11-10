[`super-forwarding-method`](../src/rules/logic/super_forwarding_method.md) (SMP527) no longer reports a method whose
name the class body reads or binds again, such as `__str__ = describe` or an import under an `if`. Deleting such a
method made the class body look the name up in the module, so the fix broke the import with a `NameError`.
