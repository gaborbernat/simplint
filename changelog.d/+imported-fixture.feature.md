New rule [`imported-fixture`](../src/rules/testing/imported_fixture.md) (SMP822) reports a pytest fixture that another
module imports or `__all__` lists; share fixtures through a `conftest.py` or a plugin. `missing-from-all` no longer asks
to list an imported fixture in `__all__`.
