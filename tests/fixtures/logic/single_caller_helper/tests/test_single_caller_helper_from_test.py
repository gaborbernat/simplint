def test_branching_helper() -> None:
    assert _expected("a") == 1


def _expected(kind: str) -> int:
    if kind == "a":
        return 1
    return 2


def test_branch_free_helper() -> None:
    assert _strip(" a ") == "a"


def _strip(text: str) -> str:
    first = text.splitlines()[0]
    return first.strip()


def branching_caller(kind: str) -> int:
    return _choose(kind)


def _choose(kind: str) -> int:
    if kind == "a":
        return 1
    return 2


def test_comprehension_item() -> None:
    assert _labels(["a", None]) == ["a", "-"]


def _labels(tags: list[str | None]) -> list[str]:
    return [tag if tag is not None else "-" for tag in tags]


def test_conditional_expression() -> None:
    assert _sign(1) == 1


def _sign(value: int) -> int:
    return 1 if value > 0 else -1


def test_match() -> None:
    assert _kind("a") == 1


def _kind(kind: str) -> int:
    match kind:
        case "a":
            return 1
        case _:
            return 2
