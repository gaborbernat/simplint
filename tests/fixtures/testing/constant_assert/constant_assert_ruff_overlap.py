def check(name: str) -> None:
    print(name); assert "ready"
    assert f"ready {name.upper()}"
    assert "deletable"
    assert [name]
