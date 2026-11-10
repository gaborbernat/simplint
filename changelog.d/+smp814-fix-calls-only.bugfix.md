[`mock-assertion-typo`](../src/rules/testing/mock_assertion_typo.md) only fixes a misspelled assertion called on a line
of its own or as the whole test of an `assert`, and drops that `assert`. Elsewhere the rename left a bound method that
is always true, or a `None` the code went on to use.
