`mock-without-spec`, `mock-attribute-assignment`, `parametrize-without-id`, the dataclass rules, `missing-final` and
`callable-ellipsis` resolve `patch`, `pytest.mark.parametrize`, `pytest.param`, `dataclass`, `TypeVar` and `Callable`
through the module's imports, as Ruff does, instead of matching the last part of the name.
