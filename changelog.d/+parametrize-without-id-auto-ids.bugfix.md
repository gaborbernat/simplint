[`parametrize-without-id`](../src/rules/testing/parametrize_without_id.md) no longer reports bytes rows and rows
unpacked into several arguments whose values are all strings, numbers, `True`, `False` or `None`, since pytest already
names them, such as `test_lower[FALSE-false]`.
