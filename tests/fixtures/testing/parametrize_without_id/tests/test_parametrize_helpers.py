import contextlib
import functools

import pytest

from tests.shared_rows import shared_row


def _row(name: str) -> pytest.ParameterSet:
    return pytest.param(name, id=f"row-{name}")


def _tuple_row(name: str) -> tuple[str, str]:
    return (name, name)


def _partial_row(name: str) -> pytest.ParameterSet:
    if name:
        return pytest.param(name, id=name)
    return pytest.param(name)


def _none_id_row(name: str) -> pytest.ParameterSet:
    return pytest.param(name, id=None)


def _no_return(name: str) -> None:
    pass


CONSTANT_ROW = _row("c")


@pytest.mark.parametrize("name", [_row("a"), _row("b"), CONSTANT_ROW])
def test_helper_with_ids(name: str) -> None:
    assert name


@pytest.mark.parametrize("name", [_partial_row("a")])
def test_one_return_without_id(name: str) -> None:
    assert name


@pytest.mark.parametrize("name", [_tuple_row("a")])
def test_helper_returning_tuple(name: str) -> None:
    assert name


@pytest.mark.parametrize("name", [_none_id_row("a"), _no_return("b")])
def test_helper_with_none_id_or_no_return(name: str) -> None:
    assert name


@pytest.mark.parametrize("name", [shared_row("a")])
def test_imported_helper(name: str) -> None:
    assert name


_lambda_row = lambda name: pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_lambda_row("a")])
def test_lambda_helper(name: str) -> None:
    assert name


def test_nested_helper() -> None:
    def _nested(name: str) -> pytest.ParameterSet:
        return pytest.param(name, id=name)

    @pytest.mark.parametrize("name", [_nested("a")])
    def test_nested(name: str) -> None:
        assert name


def _rebound_row(name: str) -> pytest.ParameterSet:
    return pytest.param(name, id=name)


def _rebound_row(name: str) -> pytest.ParameterSet:
    return pytest.param(name, id=f"{name}-again")


@pytest.mark.parametrize("name", [_rebound_row("a")])
def test_rebound_helper(name: str) -> None:
    assert name


async def _async_row(name: str) -> pytest.ParameterSet:
    return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_async_row("a")])
def test_async_helper(name: str) -> None:
    assert name


def _yielding_row(name: str) -> object:
    yield pytest.param(name, id=name)
    return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_yielding_row("a")])
def test_yielding_helper(name: str) -> None:
    assert name


def _fall_through(name: str) -> pytest.ParameterSet:
    if name:
        return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_fall_through("a")])
def test_fall_through_helper(name: str) -> None:
    assert name


def _try_row(name: str) -> pytest.ParameterSet:
    try:
        return pytest.param(name, id=name)
    except ValueError:
        pass


@pytest.mark.parametrize("name", [_try_row("a")])
def test_try_helper(name: str) -> None:
    assert name


def _loop_row(name: str) -> pytest.ParameterSet:
    for _ in range(1):
        return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_loop_row("a")])
def test_loop_helper(name: str) -> None:
    assert name


def _wrap(function):
    return function


@_wrap
def _decorated_row(name: str) -> pytest.ParameterSet:
    return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_decorated_row("a")])
def test_decorated_helper(name: str) -> None:
    assert name


def _global_row(name: str) -> pytest.ParameterSet:
    return pytest.param(name, id=name)


def _rebind_global_row() -> None:
    global _global_row
    _global_row = _row


@pytest.mark.parametrize("name", [_global_row("a")])
def test_global_rebound_helper(name: str) -> None:
    assert name


@functools.cache
def _cached_row(name: str) -> pytest.ParameterSet:
    return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_cached_row("a")])
def test_cached_helper(name: str) -> None:
    assert name


@functools.lru_cache(maxsize=None)
def _lru_row(name: str) -> pytest.ParameterSet:
    return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_lru_row("a")])
def test_lru_cache_helper(name: str) -> None:
    assert name


def _nested_loop_row(name: str) -> pytest.ParameterSet:
    def _unused() -> None:
        for _ in range(1):
            pass

    return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_nested_loop_row("a")])
def test_nested_loop_helper(name: str) -> None:
    assert name


@pytest.mark.parametrize("name", [_missing_row("a")])
def test_undefined_helper(name: str) -> None:
    assert name


def _try_finally_row(name: str) -> pytest.ParameterSet:
    try:
        return pytest.param(name, id=name)
    finally:
        pass


@pytest.mark.parametrize("name", [_try_finally_row("a")])
def test_try_finally_helper(name: str) -> None:
    assert name


def _try_returning_except_row(name: str) -> pytest.ParameterSet:
    try:
        return pytest.param(name, id=name)
    except ValueError:
        return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_try_returning_except_row("a")])
def test_try_returning_except_helper(name: str) -> None:
    assert name


def _try_reraising_row(name: str) -> pytest.ParameterSet:
    try:
        return pytest.param(name, id=name)
    except ValueError:
        raise


@pytest.mark.parametrize("name", [_try_reraising_row("a")])
def test_try_reraising_helper(name: str) -> None:
    assert name


def _loop_then_return_row(name: str) -> pytest.ParameterSet:
    for _ in range(1):
        pass
    return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_loop_then_return_row("a")])
def test_loop_then_return_helper(name: str) -> None:
    assert name


def _suppress_row(name: str) -> pytest.ParameterSet:
    with contextlib.suppress(ValueError):
        return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_suppress_row("a")])
def test_suppress_helper(name: str) -> None:
    assert name


def _if_else_row(name: str) -> pytest.ParameterSet:
    if name:
        return pytest.param(name, id=name)
    elif name == "":
        return pytest.param(name, id="empty")
    else:
        return pytest.param(name, id="other")


@pytest.mark.parametrize("name", [_if_else_row("a")])
def test_if_else_helper(name: str) -> None:
    assert name


def _match_row(name: str) -> pytest.ParameterSet:
    match name:
        case "a":
            return pytest.param(name, id=name)
        case _:
            return pytest.param(name, id="other")


@pytest.mark.parametrize("name", [_match_row("a")])
def test_match_helper(name: str) -> None:
    assert name


_NULL_SCOPE = contextlib.nullcontext()


def _null_scope_row(name: str) -> pytest.ParameterSet:
    with _NULL_SCOPE:
        return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_null_scope_row("a")])
def test_null_scope_helper(name: str) -> None:
    assert name


def _bound_guard_row(name: str) -> pytest.ParameterSet:
    guard = contextlib.suppress(ValueError)
    with guard:
        return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_bound_guard_row("a")])
def test_bound_guard_helper(name: str) -> None:
    assert name


def _raises_row(name: str) -> pytest.ParameterSet:
    with pytest.raises(ValueError):
        return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_raises_row("a")])
def test_raises_helper(name: str) -> None:
    assert name


def _exit_stack_row(name: str) -> pytest.ParameterSet:
    with contextlib.ExitStack():
        return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_exit_stack_row("a")])
def test_exit_stack_helper(name: str) -> None:
    assert name


def _async_exit_stack_row(name: str) -> pytest.ParameterSet:
    with contextlib.AsyncExitStack():
        return pytest.param(name, id=name)


@pytest.mark.parametrize("name", [_async_exit_stack_row("a")])
def test_async_exit_stack_helper(name: str) -> None:
    assert name


def _else_row(name: str) -> pytest.ParameterSet:
    try:
        text = name.upper()
    except AttributeError:
        return pytest.param(name, id="raw")
    else:
        return pytest.param(text, id="upper")


def _matched_row(name: str) -> pytest.ParameterSet:
    match name:
        case "a":
            return pytest.param(name, id="first")
        case _:
            return pytest.param(name, id="other")


@pytest.mark.parametrize("name", [_else_row("a"), _matched_row("b")])
def test_helpers_through_else_and_match(name: str) -> None:
    assert name


def _open_match_row(name: str) -> pytest.ParameterSet:
    match name:
        case "a":
            return pytest.param(name, id="first")
        case "b":
            return pytest.param(name, id="second")


@pytest.mark.parametrize("name", [_open_match_row("b")])
def test_helpers_through_open_match(name: str) -> None:
    assert name
