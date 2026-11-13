def test_unpacked_rows(rows: list[dict[str, int]]) -> None:
    assert {**(row if row else {}) for row in rows}
