def test_attributes(result):
    assert result.name == "x"
    assert result.size == 3


def test_keys(data):
    assert data["a"] == 1
    assert data["b"] == 2


def test_three_fields(result):
    assert result.name == "x"
    assert result.size == 3
    assert result.color == "red"


def test_message(result):
    assert result.name == "x", "name"
    assert result.size == 3


def test_other_comparison(result):
    assert result.name != "x"
    assert result.size > 3


def test_single(result):
    assert result.name == "x"


def test_separated(result):
    assert result.name == "x"
    print(result)
    assert result.size == 3


def test_other_objects(result, other):
    assert result.name == "x"
    assert other.size == 3


def test_long_line(result):
    assert result.some_really_long_attribute_name_here == "a value that is long enough to exceed"
    assert result.another_really_long_attribute_name == "another value that makes it too long"


def test_comment(result):
    assert result.name == "x"  # the name
    assert result.size == 3


def test_multiline_string(result):
    assert result.text == """first
second"""
    assert result.size == 3
