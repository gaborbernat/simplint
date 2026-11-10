def check(case):
    return case


def test_shared_name():
    value = 1
    table = [value, value]
    for case in table:
        assert check(case)
