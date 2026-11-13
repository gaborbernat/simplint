import itertools
import time
import unittest

CASES = [(2, 4), (3, 9)]
EXTRA = [(4, 16)]


def test_literal() -> None:
    for number, square in [(2, 4), (3, 9)]:
        assert square_of(number) == square


def test_constant() -> None:
    for number, square in CASES:
        assert square_of(number) == square


def test_local_table() -> None:
    cases = {"two": 2, "three": 3}
    for name, number in cases.items():
        assert parse(name) == number


def test_annotated_table() -> None:
    cases: tuple[str, ...] = ("a", "b")
    for case in sorted(cases):
        assert parse(case)


def test_builders() -> None:
    for index, (number, square) in enumerate(CASES + EXTRA, 1):
        assert square_of(number) == square + index - index


def test_product() -> None:
    for left, right in itertools.product((1, 2), "ab"):
        assert combine(left, right)


def test_split() -> None:
    for word in "one two".split():
        assert parse(word)


def test_subscript() -> None:
    for number, square in CASES[1:]:
        assert square_of(number) == square


def test_dict_unpacking() -> None:
    for key in {**{"a": 1}, "b": -2}:
        assert parse(key)


def test_starred() -> None:
    for value in [*CASES, (None, ...)]:
        assert check(value)


def test_bytes() -> None:
    for value in (b"a", True):
        assert check(value)


def test_unpacked_target() -> None:
    for *_, last in [(CASES[0], 1 + 2)]:
        assert parse(last)


def test_with_block() -> None:
    for name in ("a", "b"):
        with lock:
            assert parse(name)


def test_statement_reads_case() -> None:
    for number in (1, 2):
        result = square_of(number)
        assert result


def test_nested_loops() -> None:
    for row in (1, 2):
        for column in (1, 2):
            assert cell(row, column) is not None


def test_branch_in_loop() -> None:
    for number in (1, 2):
        if number:
            assert square_of(number) > 0


def test_inner_loop_reported() -> None:
    for result in compute():
        for case in ("a", "b"):
            assert result.has(parse(case))


def test_computed_items() -> None:
    for item in compute():
        assert item.valid


def test_parameter(cases: list[int]) -> None:
    for case in cases:
        assert square_of(case)


def test_attribute() -> None:
    for name in module.names:
        assert parse(name)


def test_range() -> None:
    for number in range(3):
        assert square_of(number) >= 0


def test_filled_list() -> None:
    cases = []
    cases.append(compute())
    for case in cases:
        assert square_of(case)


def test_filled_dict() -> None:
    cases = {}
    cases["a"] = compute()
    for case in cases:
        assert parse(case)


def test_rebound() -> None:
    cases = [1]
    cases = compute()
    for case in cases:
        assert square_of(case)


def test_self_reference() -> None:
    cases = [cases]
    for case in cases:
        assert square_of(case)


def test_unknown_call() -> None:
    for case in compute((1, 2)):
        assert square_of(case)


def test_unknown_builder_argument() -> None:
    for case in sorted(compute()):
        assert square_of(case)


def test_unknown_keyword() -> None:
    for case in sorted((1, 2), key=compute()):
        assert square_of(case)


def test_unknown_element() -> None:
    for case in (compute(), 2):
        assert square_of(case)


def test_unknown_operand() -> None:
    for case in CASES + compute():
        assert square_of(case)


def test_unknown_value() -> None:
    for case in (f"{compute()}", 2):
        assert square_of(case)


def test_attribute_target() -> None:
    for holder.value in (1, 2):
        assert square_of(holder.value)


def test_repeats() -> None:
    for _ in ("first", "second"):
        assert square_of(2) == 4


def test_membership() -> None:
    names = parse("a b")
    for name in ("a", "b"):
        assert name in names


def test_else() -> None:
    for name in ("a", "b"):
        if parse(name):
            break
    else:
        assert False


async def test_async_for() -> None:
    async for case in stream():
        assert square_of(case)


def test_while() -> None:
    count = 0
    while count < 3:
        count += 1
        assert count


def test_polling() -> None:
    for delay in (0.1, 0.2):
        try:
            assert ready()
        except AssertionError:
            time.sleep(delay)
        else:
            break


def test_handler_asserts() -> None:
    for number in (1, 2):
        try:
            square_of(number)
        except ValueError:
            assert number


def test_builds_data() -> None:
    values = []
    for number in (1, 2):
        values.append(square_of(number))
    assert values


def test_nested_function() -> None:
    for number in (1, 2):

        def check() -> None:
            assert number >= 0

        register(check)


def test_nested_class() -> None:
    for number in (1, 2):

        class Check:
            assert number >= 0

        register(Check)


def test_comprehension() -> None:
    assert all(square_of(number) for number in (1, 2))


def test_subtests(subtests: object) -> None:
    for number in (1, 2):
        with subtests.test(number=number):
            assert square_of(number)


class TestCase(unittest.TestCase):
    def test_method(self) -> None:
        for number in (1, 2):
            self.assertTrue(square_of(number))

    def test_raises(self) -> None:
        for number in (-1, -2):
            self.assertRaises(ValueError, square_of, number)

    def test_compares_result(self) -> None:
        names = parse("a b")
        for name in ("a", "b"):
            self.assertIn(name, names)

    def test_subtest(self) -> None:
        for number in (1, 2):
            with self.subTest(number=number):
                self.assertTrue(square_of(number))

    def test_subtest_and_assertion(self) -> None:
        for number in (1, 2):
            self.assertTrue(square_of(number))
            with self.subTest(number=number):
                self.assertTrue(square_of(number))

    def test_other_receiver(self) -> None:
        for number in (1, 2):
            helper.assertTrue(square_of(number))

    def helper_method(self) -> None:
        for number in (1, 2):
            assert square_of(number)


def helper() -> None:
    for number in (1, 2):
        assert square_of(number)


for module_number in (1, 2):
    assert square_of(module_number)


def test_set_and_attribute() -> None:
    for value in {(1).real, 2}:
        assert check(value)
