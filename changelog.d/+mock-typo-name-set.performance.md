[`mock-assertion-typo`](../src/rules/testing/mock_assertion_typo.md) checks each misspelled name against a set of the
file's function names instead of scanning every function for each one, so 32,000 misspelled calls take 0.14 seconds
instead of 0.9.
