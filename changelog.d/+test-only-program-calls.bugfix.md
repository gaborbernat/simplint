[`test-only-parameter`](../src/rules/api/test_only_parameter.md) (SMP410) counts the calls of a module that production
code star-imports, or that has an `if __name__ == "__main__":` block, as production calls, even when a test imports it.
It no longer reports a parameter that such a module passes.
