[`definition-before-use`](../src/rules/layout/definition_before_use.md) no longer pins a class named in the signature of
a `pytest.fixture`, `pytest.mark`, `pytest.hookimpl`, `mock.patch` or `deprecated` function when annotations are lazy,
as none of them evaluates the annotations while the module loads.
