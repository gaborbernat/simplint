from _test_only_parameter import connect, dispatch, encode, main, orphan, send


def test_calls() -> None:
    assert send("x", flush=True) == "x"
    assert connect("db", timeout=5) == "db:5"
    assert orphan(1, scale=2) == 2
    assert dispatch("x", verbose=True) == "x"
    assert encode("x", strict=True, ascii_only=True) == "X"
    assert main(["a"]) == 1
