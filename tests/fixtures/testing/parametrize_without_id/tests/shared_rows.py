import pytest

SHARED_ARGS = [(21, [22])]


def shared_row(name: str) -> pytest.ParameterSet:
    return pytest.param(name, id=name)
